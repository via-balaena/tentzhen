"""The LittleBit layer (docs/littlebit.md, "Model"), written from the paper's text (arXiv
2506.13771v5, section 3), none of its code, which is CC BY-NC. A weight matrix W, out x in, is
replaced by two binary factors and three scale vectors:

    y = h * (sign(U) (l * (sign(V)^T (g * x)))) + b

U (out x r) and V (in x r) are kept in float as latent weights: only their signs reach the
output. This is the paper's Proposition 1 (Eq. 5), in its order, with a bias added.

Also the two quantizers that make training see what the integer hardware computes (the recipe's
steps 3 and 4): sign() with a straight-through gradient, and uint8 activations on a power-of-two
grid. And E3's layer, binary weights with one scale per output (BinaryLinear), which uses the same
sign().
"""

import math

import torch
from torch import nn


class Sign(torch.autograd.Function):
    """+1 where w >= 0 and -1 elsewhere. Backward, the gradient is passed where |w| <= 1 and
    stopped elsewhere: the straight-through estimator of the recipe's step 3. The paper's
    SmoothSign is not used."""

    @staticmethod
    def forward(ctx, w):
        ctx.save_for_backward(w)
        return torch.where(w >= 0, torch.ones_like(w), -torch.ones_like(w))

    @staticmethod
    def backward(ctx, grad):
        (w,) = ctx.saved_tensors
        return grad * (w.abs() <= 1).to(grad.dtype)


class Uint8(torch.autograd.Function):
    """floor(y / 2^e), clamped to 0..255, times 2^e: what the hardware's arithmetic right shift,
    ReLU and clamp to uint8 give (docs/littlebit.md, "Fixed-point formats"). Dividing by a power
    of two is exact in float, so each value is exactly a uint8 times 2^e. Backward, the gradient
    is passed where y is inside the grid's range, 0 <= y < 256 * 2^e, and stopped where y is
    clamped."""

    @staticmethod
    def forward(ctx, y, exponent):
        step = 2.0**exponent
        ctx.save_for_backward(y)
        ctx.top = 256 * step
        return (y / step).floor().clamp(0, 255) * step

    @staticmethod
    def backward(ctx, grad):
        (y,) = ctx.saved_tensors
        return grad * ((y >= 0) & (y < ctx.top)).to(grad.dtype), None


class Activations(nn.Module):
    """A layer's outputs as uint8 times 2^exponent. The exponent is set before training
    (`exponent_for`) and is not a parameter."""

    def __init__(self):
        super().__init__()
        self.exponent = None

    def forward(self, y):
        return Uint8.apply(y, self.exponent)

    def extra_repr(self):
        return f"exponent={self.exponent}"


def exponent_for(largest):
    """The smallest e for which `largest` is below 256 * 2^e, so floor(largest / 2^e) <= 255 and
    nothing up to it is clamped."""
    if not largest > 0:
        raise ValueError(f"no positive output to fit a grid to: the largest is {largest}")
    return math.frexp(largest / 256)[1]


class LittleBitLinear(nn.Module):
    """W ~ diag(h) sign(U) diag(l) sign(V)^T diag(g), plus a bias. The parameters are registered
    in the order the input meets them: g, v, l, u, h, bias."""

    def __init__(self, n_in, n_out, rank):
        super().__init__()
        self.rank = rank
        self.g = nn.Parameter(torch.empty(n_in))
        self.v = nn.Parameter(torch.empty(n_in, rank))
        self.l = nn.Parameter(torch.empty(rank))
        self.u = nn.Parameter(torch.empty(n_out, rank))
        self.h = nn.Parameter(torch.empty(n_out))
        self.bias = nn.Parameter(torch.empty(n_out))

    @classmethod
    def from_dense(cls, weight, bias, rank):
        """The layer Dual-SVID starts from `weight` (out x in) at rank `rank`, with `bias` kept."""
        n_out, n_in = weight.shape
        layer = cls(n_in, n_out, rank)
        u, v, h, l, g = dual_svid(weight, rank)
        with torch.no_grad():
            for p, value in [(layer.g, g), (layer.v, v), (layer.l, l), (layer.u, u), (layer.h, h)]:
                p.copy_(value)
            layer.bias.copy_(bias)
        return layer

    def forward(self, x):
        t = (x * self.g) @ Sign.apply(self.v)
        t = t * self.l
        t = t @ Sign.apply(self.u).T
        return t * self.h + self.bias

    def dense(self):
        """The weight matrix the factors make, out x in: diag(h) sign(U) diag(l) sign(V)^T diag(g),
        with sign()'s gradient as in `forward`."""
        return (Sign.apply(self.u) * self.l) @ Sign.apply(self.v).T * self.h[:, None] * self.g

    def extra_repr(self):
        return f"in={self.g.numel()}, out={self.h.numel()}, rank={self.rank}"


class LittleBitConv2d(nn.Module):
    """A convolution at stride 1 with "same" padding, whose kernel, as a matrix out x (in x kernel
    x kernel), is a LittleBit layer's (`LittleBitLinear`). It convolves with the dense kernel the
    factors make, which computes what applying the factors to each window would."""

    def __init__(self, n_in, n_out, kernel, rank):
        super().__init__()
        self.shape = (n_out, n_in, kernel, kernel)
        self.linear = LittleBitLinear(n_in * kernel * kernel, n_out, rank)

    @classmethod
    def from_dense(cls, weight, bias, rank):
        """Dual-SVID's start from a kernel `weight` (out x in x kernel x kernel) and its bias."""
        n_out, n_in, kernel, _ = weight.shape
        layer = cls(n_in, n_out, kernel, rank)
        layer.linear = LittleBitLinear.from_dense(weight.reshape(n_out, -1), bias, rank)
        return layer

    def dense(self):
        return self.linear.dense().reshape(self.shape)

    def forward(self, x):
        return nn.functional.conv2d(x, self.dense(), self.linear.bias, padding=self.shape[2] // 2)

    def extra_repr(self):
        n_out, n_in, kernel, _ = self.shape
        return f"in={n_in}, out={n_out}, kernel={kernel}, rank={self.linear.rank}"


class BinaryLinear(nn.Module):
    """E3's layer: W ~ diag(h) sign(W), one scale per output, plus a bias, so
    y = h * (sign(W) x) + b. W, out x in, is kept in float as latent weights: only its signs reach
    the output. The parameters are registered in the order the input meets them: weight, h,
    bias."""

    def __init__(self, n_in, n_out):
        super().__init__()
        self.weight = nn.Parameter(torch.empty(n_out, n_in))
        self.h = nn.Parameter(torch.empty(n_out))
        self.bias = nn.Parameter(torch.empty(n_out))

    @classmethod
    def from_dense(cls, weight, bias):
        """The layer started from a dense `weight` (out x in) and its `bias`: the latent weights
        are `weight` itself, and each h the mean of |weight| along its output's row, the scale
        that brings h sign(W) closest to W in squared error."""
        n_out, n_in = weight.shape
        layer = cls(n_in, n_out)
        with torch.no_grad():
            layer.weight.copy_(weight)
            layer.h.copy_(weight.abs().mean(1))
            layer.bias.copy_(bias)
        return layer

    def forward(self, x):
        return (x @ Sign.apply(self.weight).T) * self.h + self.bias

    def extra_repr(self):
        return f"in={self.weight.shape[1]}, out={self.weight.shape[0]}"


def dual_svid(weight, rank):
    """Dual-SVID (the paper's section 3.2, Eqs. 6-8): W's truncated SVD gives U' and V' with
    W ~ U' V'^T; their signs start the factors, and U and V start as U' and V' themselves. A
    rank-one fit of |U'| gives h and l_u, one of |V'| gives g and l_v, and l = l_u * l_v.

    The paper does not say how the singular values are shared between U' and V', nor how each
    rank-one fit is split between its two vectors. Here each side takes the square root, both
    times. Computed in float64 on the CPU, returned as float32."""
    w = weight.detach().cpu().double()
    left, s, right = torch.linalg.svd(w, full_matrices=False)
    root = s[:rank].sqrt()
    u = left[:, :rank] * root
    v = right[:rank].T * root
    h, l_u = _rank_one(u.abs())
    g, l_v = _rank_one(v.abs())
    return [t.float() for t in (u, v, h, l_u * l_v, g)]


def _rank_one(m):
    """a and b with m ~ a b^T, from m's leading singular triple (s, x, y): a = sqrt(s) x and
    b = sqrt(s) y. m is non-negative, so x and y are taken with the sign that makes them so."""
    left, s, right = torch.linalg.svd(m, full_matrices=False)
    x, y = left[:, 0], right[0]
    if x.sum() < 0:
        x, y = -x, -y
    if (x < 0).any() or (y < 0).any():
        raise ValueError("the leading singular vectors of a non-negative matrix came out mixed")
    root = s[0].sqrt()
    return root * x, root * y
