"""A LittleBit layer as a hardwired kernel (docs/littlebit.md, "A hardwired kernel for conv1"):
every integer of the layer a constant of the circuit, and a whole input vector, a convolution's
window, taken each clock. The layer engine (`littlebit.py`) takes one input a clock and multiplies
by scales it reads from tables. Here nothing is read and nothing multiplies: every scale is folded
into the logic.

It computes what the golden model computes (`LittleBitLayer::trace` in
crates/littlebit/src/integer.rs), the same as the engine, in this order:

1. V. Input i is `widths[i]` bits wide, and each of its bits b stands for g_i * 2^b. Each input's
   row of V is a pattern of `rank` signs, and its sign for latent 0 is made + by negating the row
   and g_i together if it isn't: that leaves at most 2^(rank - 1) patterns, the classes. Within a
   class, every 4 bits index a table of the 16 sums their weights make, so that each bit of the
   sum is one LUT4 of the ECP5. The class's tables are summed in a tree of adders, and each
   latent a_k is the class sums, each added or subtracted by its pattern's sign k. An input is in
   one class, so each bit is added once, not once a latent.
2. ℓ. t_k = (a_k * l_k) >> latent_shift, saturated to an int16. The multiply by l_k is shifts and
   adds, by l_k's canonical signed digits, so no multiplier is used.
3. U and h. s_o is the latents, each added or subtracted by U's row o; z_o = s_o * h_o + bias_o,
   by h_o's digits as in 2; y_o = z_o >> output_shift, clamped to 0..255.

Every right shift is arithmetic, as the golden model's are. Each value is as wide as the bounds
its inputs give it, so none wraps. The adder trees keep a register after every `every` levels of
adds, and after each table and each stage, and the kernel takes a new vector every clock: a
vector's outputs, all of them together, come out `latency` clocks after it went in, with
`y_valid`. A layer whose a, s or z could pass an int32 is refused, as the golden model refuses
one whose values do.

That the kernel matches the golden model, vector for vector and at its latency, is `tested` on
the test vectors' Clean Up conv1 case (tests/test_littlebit_kernel.py). What it costs in LUTs, and
how fast it closes, are nextpnr's to report: `python -m gateware.cores.littlebit_kernel` writes a
kernel as Verilog for Yosys and nextpnr, alone or in `Alone`, which loads a vector a bit a clock.
"""

from amaranth import Cat, Const, Elaboratable, Module, Mux, Shape, Signal, signed

from gateware.cores.littlebit import ROM

__all__ = ["GROUP", "INT32", "csd", "WindowKernel", "Alone", "channel_widths"]

# The bits one table takes: an ECP5 LUT has 4 inputs.
GROUP = 4
T_MIN, T_MAX = -(1 << 15), (1 << 15) - 1
INT32 = (-(1 << 31), (1 << 31) - 1)


def csd(c):
    """The canonical signed digits of the integer `c`: (e, d) pairs, d +1 or -1, whose d * 2^e sum
    to c, no two digits next to each other."""
    digits, e = [], 0
    while c:
        if c & 1:
            d = 2 - (c & 3)
            digits.append((e, d))
            c -= d
        c >>= 1
        e += 1
    return digits


def channel_widths(inputs, channels, byte_channels, order):
    """Each input's width in bits, for a convolution's window of `inputs` over `channels`: 8 for
    a channel in `byte_channels`, 1 for the others. `order` is how the window is flattened:
    "rows" for row, column and then channel, as the golden model's `windows` does, or "channels"
    for channel first, as PyTorch's convolution weights are."""
    if inputs % channels:
        raise ValueError(f"{inputs} inputs are not a whole number of {channels} channels")
    if order == "rows":
        of = [i % channels for i in range(inputs)]
    elif order == "channels":
        of = [i // (inputs // channels) for i in range(inputs)]
    else:
        raise ValueError(f"order {order!r} is neither rows nor channels")
    return [8 if c in byte_channels else 1 for c in of]


class _Value:
    """A signal, the bounds of the integer it holds, and the clock it is valid on, counted in
    registers from the kernel's input."""

    def __init__(self, signal, low, high, stage):
        self.signal, self.low, self.high, self.stage = signal, low, high, stage


class _Builder:
    """Bounded arithmetic in a module: each sum is a signal just wide enough for its bounds, and
    values from different clocks are delayed to the later before they meet."""

    def __init__(self, m):
        self.m = m
        self.count = 0

    def _signal(self, low, high, name):
        self.count += 1
        return Signal(range(low, high + 1), name=f"{name}_{self.count}")

    def constant(self, c):
        return _Value(Const(c, Shape.cast(range(min(c, 0), max(c, 0) + 1))), c, c, None)

    def register(self, v, name="r"):
        if v.stage is None:
            return v
        s = self._signal(v.low, v.high, name)
        self.m.d.sync += s.eq(v.signal)
        return _Value(s, v.low, v.high, v.stage + 1)

    def at(self, v, stage):
        while v.stage is not None and v.stage < stage:
            v = self.register(v, "d")
        return v

    def comb(self, expression, low, high, stage, name):
        s = self._signal(low, high, name)
        self.m.d.comb += s.eq(expression)
        return _Value(s, low, high, stage)

    def negate(self, v):
        if v.stage is None:
            return self.constant(-v.low)
        return self.comb(-v.signal, -v.high, -v.low, v.stage, "n")

    def shift_left(self, v, e):
        if e == 0:
            return v
        return self.comb(v.signal << e, v.low << e, v.high << e, v.stage, "sl")

    def add(self, a, b):
        stages = [s for s in (a.stage, b.stage) if s is not None]
        stage = max(stages) if stages else None
        if stage is None:
            return self.constant(a.low + b.low)
        a, b = self.at(a, stage), self.at(b, stage)
        return self.comb(a.signal + b.signal, a.low + b.low, a.high + b.high, stage, "s")

    def tree(self, terms, every):
        """The sum of `terms`, a balanced tree of adds with a register after every `every`
        levels."""
        level = 0
        while len(terms) > 1:
            pairs = [self.add(a, b) for a, b in zip(terms[0::2], terms[1::2])]
            terms = pairs + terms[len(terms) & ~1 :]
            level += 1
            if level % every == 0:
                terms = [self.register(t) for t in terms]
        return terms[0]

    def times(self, v, c, every):
        """v * c, for a constant c, by c's canonical signed digits."""
        if c == 0:
            return self.constant(0)
        terms = []
        for e, d in csd(c):
            t = self.shift_left(v, e)
            terms.append(t if d > 0 else self.negate(t))
        product = self.tree(terms, every)
        # The tree's bounds are looser than the product's.
        low, high = sorted((v.low * c, v.high * c))
        return self.narrow(product, low, high)

    def narrow(self, v, low, high):
        if v.stage is None or (v.low, v.high) == (low, high):
            return v
        return self.comb(v.signal, low, high, v.stage, "w")

    def floor_shift(self, v, shift):
        """v >> shift, arithmetic."""
        if shift == 0:
            return v
        return self.comb(v.signal >> shift, v.low >> shift, v.high >> shift, v.stage, "sr")

    def clamp(self, v, low, high):
        if v.low >= low and v.high <= high:
            return v
        clamped = Mux(v.signal > high, high, Mux(v.signal < low, low, v.signal))
        return self.comb(clamped, max(v.low, low), min(v.high, high), v.stage, "c")


def _refuse_past_int32(what, low, high):
    if low < INT32[0] or high > INT32[1]:
        raise ValueError(f"{what} could reach {low} or {high}, past an int32")


class WindowKernel(Elaboratable):
    """One LittleBit layer, `layer` as integer.json holds it (a dict), applied to a whole vector a
    clock: input i is `widths[i]` bits, from 1 to 8, of `x`, the inputs packed in order from bit
    0. A value at or past 2^widths[i] can't be given, so the kernel is the layer only for inputs
    that fit their widths.

    In: `x`, taken on every clock `x_valid` is high. Out: `latency` clocks later, `y`, the outputs
    packed 8 bits each from bit 0, `z`, each output's int32 before the shift and clamp, packed 32
    bits each, for tests to compare, and `y_valid`. `every` is how many levels of adds go between
    registers in a tree."""

    def __init__(self, layer, widths, every=2):
        self.inputs, self.outputs, self.rank = layer["inputs"], layer["outputs"], layer["rank"]
        if len(widths) != self.inputs or any(not 1 <= w <= 8 for w in widths):
            raise ValueError("widths needs one width a layer input, each from 1 to 8 bits")
        if len(layer["g"]) != self.inputs or len(layer["v"]) != self.inputs:
            raise ValueError("g or V is not as long as the layer's inputs")
        if len(layer["l"]) != self.rank:
            raise ValueError("l is not as long as the layer's rank")
        if any(len(x) != self.outputs for x in (layer["u"], layer["h"], layer["bias"])):
            raise ValueError("U, h or the bias is not as long as the layer's outputs")
        if any(len(row) != self.rank for row in layer["v"] + layer["u"]):
            raise ValueError("a row of V or U is not as long as the layer's rank")
        if any(set(row) - {"+", "-"} for row in layer["v"] + layer["u"]):
            raise ValueError("a sign is + or -")
        if every < 1:
            raise ValueError("every is at least 1 level of adds")
        self.layer, self.widths, self.every = layer, list(widths), every
        self._refuse_past_int32()
        self.x = Signal(sum(widths))
        self.x_valid = Signal()
        self.y = Signal(8 * self.outputs)
        self.z = Signal(32 * self.outputs)
        self.y_valid = Signal()
        self.classes = self._classes()
        # Built once, here, so that its latency and tables are known before it is elaborated.
        self._module = self._build()

    def ports(self):
        return [self.x, self.x_valid, self.y, self.z, self.y_valid]

    def _refuse_past_int32(self):
        """Each a's, s's and z's bounds, from the layer and the widths alone, before anything is
        built: a refused layer leaves nothing behind."""
        layer = self.layer
        t = []
        for k, l in enumerate(layer["l"]):
            parts = [
                (1 if row[k] == "+" else -1) * g * ((1 << w) - 1)
                for row, g, w in zip(layer["v"], layer["g"], self.widths)
            ]
            low, high = sum(p for p in parts if p < 0), sum(p for p in parts if p > 0)
            _refuse_past_int32(f"latent {k}", low, high)
            low, high = sorted((low * l, high * l))
            shift = layer["latent_shift"]
            t.append((max(low >> shift, T_MIN), min(high >> shift, T_MAX)))
        for o, (row, h, bias) in enumerate(zip(layer["u"], layer["h"], layer["bias"])):
            low = sum(lo if sign == "+" else -hi for sign, (lo, hi) in zip(row, t))
            high = sum(hi if sign == "+" else -lo for sign, (lo, hi) in zip(row, t))
            _refuse_past_int32(f"s_{o}", low, high)
            low, high = sorted((low * h, high * h))
            _refuse_past_int32(f"z_{o}", low + bias, high + bias)

    def _classes(self):
        """Each class's pattern of signs, latent 0's +, and its members: (input, its sign)."""
        classes = {}
        for i, row in enumerate(self.layer["v"]):
            sign = 1 if row[0] == "+" else -1
            pattern = row if sign > 0 else row.translate(str.maketrans("+-", "-+"))
            classes.setdefault(pattern, []).append((i, sign))
        return classes

    def elaborate(self, platform):
        return self._module

    def _build(self):
        m = Module()
        b = _Builder(m)
        layer, every = self.layer, self.every
        offsets = [sum(self.widths[:i]) for i in range(self.inputs)]

        # 1. Each class's sum, from tables of 4 bits each, the byte-wide inputs' bits first.
        class_sums = {}
        tables = 0
        for pattern, members in self.classes.items():
            bits = []
            for i, sign in sorted(members, key=lambda member: -self.widths[member[0]]):
                c = sign * layer["g"][i]
                if c:
                    bits += [(offsets[i] + j, c << j) for j in range(self.widths[i])]
            terms = []
            for start in range(0, len(bits), GROUP):
                group = bits[start : start + GROUP]
                rows = [
                    sum(w for j, (_, w) in enumerate(group) if index >> j & 1)
                    for index in range(1 << len(group))
                ]
                low, high = min(rows), max(rows)
                tables += 1
                m.submodules[f"table_{tables}"] = rom = ROM(rows, Shape.cast(range(low, high + 1)))
                m.d.comb += rom.index.eq(Cat(*(self.x[bit] for bit, _ in group)))
                terms.append(b.register(_Value(rom.data, low, high, 0), "table"))
            class_sums[pattern] = b.tree(terms, every) if terms else b.constant(0)
        self.tables = tables

        latents = []
        for k in range(self.rank):
            terms = [
                total if pattern[k] == "+" else b.negate(total)
                for pattern, total in class_sums.items()
            ]
            latents.append(b.register(b.tree(terms, every), "a") if terms else b.constant(0))

        # 2. t_k, saturated to an int16.
        t = []
        for a, l in zip(latents, layer["l"]):
            product = b.times(a, l, every)
            t.append(b.register(b.clamp(b.floor_shift(product, layer["latent_shift"]),
                                        T_MIN, T_MAX), "t"))

        # 3. s_o, then z_o and y_o.
        zs, ys = [], []
        for o, (row, h, bias) in enumerate(zip(layer["u"], layer["h"], layer["bias"])):
            terms = [t_k if sign == "+" else b.negate(t_k) for sign, t_k in zip(row, t)]
            s = b.register(b.tree(terms, every), "s_o")
            z = b.tree([b.times(s, h, every), b.constant(bias)], every)
            zs.append(z)
            ys.append(b.clamp(b.floor_shift(z, layer["output_shift"]), 0, 255))

        stage = max(v.stage for v in zs + ys if v.stage is not None)
        for o, (z, y) in enumerate(zip(zs, ys)):
            z, y = b.at(z, stage), b.at(y, stage)
            m.d.sync += [
                self.z.word_select(o, 32).eq(z.signal),
                self.y.word_select(o, 8).eq(y.signal),
            ]
        self.latency = stage + 1

        valid = self.x_valid
        for j in range(self.latency):
            delayed = Signal(name=f"valid_{j + 1}")
            m.d.sync += delayed.eq(valid)
            valid = delayed
        m.d.comb += self.y_valid.eq(valid)
        return m


class Alone(Elaboratable):
    """A kernel as its own top, for Yosys and nextpnr to size, with a handful of pins: a vector is
    shifted in a bit a clock, `x_in` on a clock `x_shift` is high, into a register as wide as the
    kernel's input, and the kernel takes it on a clock `x_valid` is high. Its outputs are kept on
    `y_valid` and shifted out a bit a clock on `y_out`. `z`, which only tests read, is left out,
    so that synthesis drops what makes it. The vector's register is one flip-flop for each of its
    bits, and the outputs' one for each of theirs, besides the kernel's."""

    def __init__(self, layer, widths, every=2):
        self.kernel = WindowKernel(layer, widths, every)
        self.x_in = Signal()
        self.x_shift = Signal()
        self.x_valid = Signal()
        self.y_out = Signal()
        self.y_valid = Signal()

    def ports(self):
        return [self.x_in, self.x_shift, self.x_valid, self.y_out, self.y_valid]

    def elaborate(self, platform):
        m = Module()
        m.submodules.kernel = k = self.kernel
        window = Signal(len(k.x))
        out = Signal(len(k.y))
        with m.If(self.x_shift):
            m.d.sync += window.eq(Cat(self.x_in, window[:-1]))
        with m.If(k.y_valid):
            m.d.sync += out.eq(k.y)
        with m.Else():
            m.d.sync += out.eq(out >> 1)
        m.d.comb += [
            k.x.eq(window),
            k.x_valid.eq(self.x_valid),
            self.y_out.eq(out[0]),
            self.y_valid.eq(k.y_valid),
        ]
        return m


if __name__ == "__main__":
    # python -m gateware.cores.littlebit_kernel <integer.json> <layer index> <out.v> <order>
    #     [kernel | alone] [every]
    # from the repo root. <order> is how the layer's windows are flattened, "rows" or "channels"
    # (`channel_widths`), over Clean Up's 19 channels, of which 16 and 17 are bytes
    # (crates/littlebit/src/vectors.rs, CLEAN_UP_BYTE_CHANNELS). The kernel is written alone, with
    # its wide ports, or in `Alone`, then, for instance,
    #   yosys -p "read_verilog <out.v>; synth_ecp5 -top kernel -json <out.json>"
    #   nextpnr-ecp5 --45k --package CABGA381 --speed 6 --freq 50 --seed 1 --json <out.json>
    import json
    import sys
    from pathlib import Path

    from amaranth.back import verilog

    path, index, out, order = sys.argv[1], int(sys.argv[2]), Path(sys.argv[3]), sys.argv[4]
    top_kind = sys.argv[5] if len(sys.argv) > 5 else "alone"
    every = int(sys.argv[6]) if len(sys.argv) > 6 else 2
    text = json.loads(Path(path).read_text())
    layer = text["layers"][index] if "layers" in text else text["cases"][index]["layer"]
    widths = channel_widths(layer["inputs"], 19, (16, 17), order)
    if top_kind == "kernel":
        top = WindowKernel(layer, widths, every)
        ports = [top.x, top.x_valid, top.y, top.y_valid]
    else:
        top = Alone(layer, widths, every)
        ports = top.ports()
    out.write_text(verilog.convert(top, name="kernel", ports=ports))
    print(f"{out}: {top_kind}, every {every}, latency "
          f"{(top if top_kind == 'kernel' else top.kernel).latency} clocks, "
          f"{(top if top_kind == 'kernel' else top.kernel).tables} tables")
