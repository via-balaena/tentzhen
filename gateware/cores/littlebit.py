"""The LittleBit layer engine (docs/littlebit.md, "Hardware architecture"): one LittleBit layer
from `integer.json`, its integers hardwired into logic (Variant A), applied to a stream of input
vectors. An MLP layer is the one-vector case; a convolution streams the window around each
position of its grid (docs/littlebit.md, "After MNIST"). Below it, the output layer,
`DenseEngine`, and a whole network, `LittleBitNetwork`, its engines one after another.

It computes what the golden model computes (`LittleBitLayer::trace` in
crates/littlebit/src/integer.rs), in three stages a vector:

1. V, one input per clock as it arrives, `inputs` clocks, and one more: p = x * g_i, and each of
   the `rank` accumulators a_k adds p, or subtracts it, by V's row i. The input and its rows of g
   and V are kept for a clock first, so each add comes a clock after its input.
2. ℓ, one latent per clock, `rank` clocks: t_k = (a_k * l_k) >> latent_shift, saturated to an
   int16 and kept, with its negation.
3. U and h, one output per clock, `outputs` clocks: U's row o is read; a clock later s_o, the sum
   of the `rank` latents, each added or subtracted by that row, in one adder tree; a clock later
   z_o = s_o * h_o + bias_o, and y_o = z_o >> output_shift, clamped to 0..255. Each output comes
   out with `y_valid`, the last with `y_last` too.

Stage 3 sums each output's latents at once, rather than keeping an accumulator for every output
and adding one latent to all of them each clock in stage 2. Synthesized for r16's first layer,
the 256 accumulators that would take came to 18,778 LUT4s and 3,106 carry cells (Yosys, OSS CAD
Suite 2026-11-21), past what both layers may use (docs/littlebit.md, "Hardware architecture").
The clocks kept between steps are where nextpnr's first routes of the two layers fell short of
50 MHz: 40.72 and 44.80 MHz.

Every right shift is arithmetic, as the golden model's are. The sums are as wide as they can get,
so none wraps. The a's shift along in stage 2, so it reads only the first, and the latents shift
in behind them. The engine takes no input in stages 2 and 3. Given an input every clock, a vector
takes inputs + rank + outputs + 1 clocks, its last output comes two clocks after that, and the
next vector's first input is taken on the first of those two.

Each engine keeps its tables one of two ways, its `variant` (docs/littlebit.md, "Variant A and
Variant B in Amaranth"). In Variant A, the ROMs, `ROM`, are trees of multiplexers on constants,
so Yosys makes them logic. In Variant B, every table whose row is kept in a register a clock
before it is used (g, V, U, h and the bias, and the output layer's weights) is the initial
contents of a memory, `StoredROM`, for Yosys to put in block RAM. A memory's read port gives its
row a clock after it is asked, so it is asked a clock early, for the index the engine will be at
next, and its row is kept in the same register as in Variant A: both take the same clocks, and
the block RAM's slow output feeds only that register. ℓ is read on the clock it is used, and
stays logic in both. That the engine matches the golden model, and takes those clocks, is
`tested` on its test vectors (tests/test_littlebit.py, from crates/littlebit/src/vectors.rs).
What it costs in LUTs, and how fast it closes, are nextpnr's to report:
`python -m gateware.cores.littlebit` writes an engine alone as Verilog, for Yosys and nextpnr.
"""

from amaranth import Array, Const, Elaboratable, Module, Mux, Signal, signed, unsigned
from amaranth.lib.memory import Memory

__all__ = [
    "ROM", "StoredROM", "VARIANTS", "LittleBitEngine", "DenseEngine", "LittleBitNetwork",
    "Alone", "sign_bits",
]

# A, the tables in logic; B, in memories, for block RAM.
VARIANTS = ("A", "B")

# The largest product of a uint8 input and an int8 scale, in size: 255 * 128.
P_MAX = 255 * 128
T_MIN, T_MAX = -(1 << 15), (1 << 15) - 1


def sign_bits(signs):
    """Signs as integer.json writes them, a string of `+` and `-`, as an integer whose bit k is 1
    where sign k is `-`."""
    if set(signs) - {"+", "-"}:
        raise ValueError(f"a sign is + or -, not one of {sorted(set(signs) - {'+', '-'})}")
    return sum(1 << k for k, c in enumerate(signs) if c == "-")


class ROM(Elaboratable):
    """Row `index` of `rows`, integers of shape `shape`, on `data`, as a tree of 2:1 multiplexers
    on the constant rows, one level for each bit of the index: the rows become logic (Variant A).
    Past the last row, `data` is 0.

    Not a Switch with a constant on each Case: Yosys's proc_rom pass turns such a Switch into a
    memory, and once the memory's output is registered, synth_ecp5 put it in block RAM. Four of
    layer 1's ROMs went into 4 DP16KD blocks that way (OSS CAD Suite 2026-11-21), which would
    make it Variant B. proc_rom works on a process's switches only (passes/proc/proc_rom.cc), and
    a tree of multiplexers is none."""

    def __init__(self, rows, shape):
        self.rows = list(rows)
        self.index = Signal(range(max(len(self.rows), 2)))
        self.data = Signal(shape)

    def elaborate(self, platform):
        m = Module()
        shape = self.data.shape()
        level = [Const(row, shape) for row in self.rows]
        for bit in range(len(self.index)):
            pairs = zip(level[0::2], level[1::2] + [Const(0, shape)] * (len(level) % 2))
            level = [Mux(self.index[bit], high, low) for low, high in pairs]
        m.d.comb += self.data.eq(level[0])
        return m


class StoredROM(Elaboratable):
    """Row `index` of `rows`, integers of shape `shape`, on `data` a clock later: the rows are the
    initial contents of a memory with one synchronous read port and no write port, which Yosys
    maps to block RAM (Variant B). `data` is the row `index` named on the clock before.

    The memory is marked `ram_style = "block"`, so that every table of Variant B is in block RAM.
    Without it, Yosys's memory_libmap puts a ROM of 2,048 bits or fewer in logic: it costs a ROM
    in logic at a sixteenth a bit (`logic_cost_rom`, passes/memory/memory_libmap.cc) and a DP16KD
    at 128 (techlibs/lattice/brams_16kd.txt), and keeps the logic unless a block costs less. Four
    of r16's tables are that small."""

    def __init__(self, rows, shape):
        self.rows = list(rows)
        self.memory = Memory(
            shape=shape, depth=max(len(self.rows), 2), init=self.rows, attrs={"ram_style": "block"}
        )
        self.port = self.memory.read_port()
        self.index = self.port.addr
        self.data = self.port.data

    def elaborate(self, platform):
        m = Module()
        m.submodules.memory = self.memory
        return m


def check_variant(variant):
    if variant not in VARIANTS:
        raise ValueError(f"variant {variant!r} is none of {', '.join(VARIANTS)}")


def table(rows, shape, variant):
    """The ROM of `rows` that `variant` makes: logic in A, a memory in B."""
    return ROM(rows, shape) if variant == "A" else StoredROM(rows, shape)


def admit(m, variant, x_valid):
    """An engine's `x_ready` while it waits for inputs, and the condition on which it takes one.
    In Variant A, always ready, and it takes an input whenever one is offered. In Variant B, ready
    from the second clock after a reset. On the clock of a reset, a memory may be asked for the
    row after the first, for an input the reset then refuses; on the clock after, it is asked for
    the first row, which it gives on the next, when the engine is first ready."""
    if variant == "A":
        return Const(1), x_valid
    primed = Signal()
    m.d.sync += primed.eq(1)
    return primed, x_valid & primed


def following(index, last, step):
    """The index a counter from 0 to `last` takes on the next clock: one more on a clock `step`
    is high, back to 0 after `last`, and the same otherwise."""
    return Mux(step, Mux(index == last, 0, index + 1), index)


def signed_sum(terms):
    """The sum of `terms`, as a balanced tree of adds."""
    while len(terms) > 1:
        terms = [a + b for a, b in zip(terms[0::2], terms[1::2])] + terms[len(terms) & ~1 :]
    return terms[0]


class LittleBitEngine(Elaboratable):
    """One LittleBit layer, `layer` as integer.json holds it (a dict), applied to each vector of
    uint8s streamed into `x`.

    In: `x`, taken on a clock when `x_valid` and `x_ready` are both high, a vector's inputs in
    order. Out: `y`, a vector's outputs in order, each on a clock `y_valid` is high, the last with
    `y_last`; and beside each, `z`, the int32 before the shift and clamp, for tests to compare.

    `variant` says where its tables are: "A" in logic, "B" in memories. Either way, each row is
    kept in a register a clock before the arithmetic it feeds. A ROM gives its row on the clock it
    is asked, so it is given the index the engine is at. A memory's read port gives its row a
    clock later, so it is given the index the engine will be at on the next clock: its row is
    then there to keep on the clock the engine keeps it. In Variant B, the engine takes no input
    on its first clock after a reset, while its memories read their first rows."""

    def __init__(self, layer, variant="A"):
        check_variant(variant)
        self.variant = variant
        self.inputs = layer["inputs"]
        self.outputs = layer["outputs"]
        self.rank = layer["rank"]
        if len(layer["g"]) != self.inputs or len(layer["v"]) != self.inputs:
            raise ValueError("g or V is not as long as the layer's inputs")
        if len(layer["l"]) != self.rank:
            raise ValueError("l is not as long as the layer's rank")
        if any(len(x) != self.outputs for x in (layer["u"], layer["h"], layer["bias"])):
            raise ValueError("U, h or the bias is not as long as the layer's outputs")
        if any(len(row) != self.rank for row in layer["v"] + layer["u"]):
            raise ValueError("a row of V or U is not as long as the layer's rank")
        self.layer = layer
        self.x = Signal(8)
        self.x_valid = Signal()
        self.x_ready = Signal()
        self.y = Signal(8)
        self.z = Signal(signed(32))
        self.y_valid = Signal()
        self.y_last = Signal()

    def ports(self):
        return [self.x, self.x_valid, self.x_ready, self.y, self.z, self.y_valid, self.y_last]

    def elaborate(self, platform):
        m = Module()
        layer, n_in, n_out, rank = self.layer, self.inputs, self.outputs, self.rank
        variant = self.variant

        # V's and U's rows, one per input and one per output: bit k is 1 where sign k is `-`.
        m.submodules.g = g = table(layer["g"], signed(8), variant)
        m.submodules.v = v = table([sign_bits(row) for row in layer["v"]], unsigned(rank), variant)
        m.submodules.l = l = ROM(layer["l"], signed(8))
        m.submodules.u = u = table([sign_bits(row) for row in layer["u"]], unsigned(rank), variant)
        m.submodules.h = h = table(layer["h"], signed(8), variant)
        m.submodules.bias = bias = table(layer["bias"], signed(32), variant)

        i = Signal(range(max(n_in, 2)))
        k = Signal(range(max(rank, 2)))
        o = Signal(range(max(n_out, 2)))

        a_max = n_in * P_MAX
        s_max = rank * (1 << 15)
        a = [Signal(range(-a_max, a_max + 1), name=f"a_{j}") for j in range(rank)]
        # Each latent, and its negation, kept for stage 3.
        t = [Signal(signed(16), name=f"t_{j}") for j in range(rank)]
        nt = [Signal(signed(17), name=f"nt_{j}") for j in range(rank)]

        # Stage 1, its first clock: the input and its rows of g and V, kept. Its second: the
        # product, a uint8 times an int8, added or subtracted.
        x_1 = Signal(8)
        g_1 = Signal(signed(8))
        v_1 = Signal(unsigned(rank))
        first_1 = Signal()
        valid_1 = Signal()
        p = Signal(signed(17))
        if variant == "A":
            m.d.comb += [g.index.eq(i), v.index.eq(i)]
        m.d.comb += p.eq(x_1 * g_1)
        m.d.sync += valid_1.eq(0)
        with m.If(valid_1):
            for j in range(rank):
                start = Mux(first_1, 0, a[j])
                m.d.sync += a[j].eq(start + Mux(v_1[j], -p, p))

        # Stage 2's latent, from the first a: the product, shifted, saturated to an int16.
        m.d.comb += l.index.eq(k)
        latent = (a[0] * l.data) >> layer["latent_shift"]
        shifted = Signal(latent.shape())
        t_next = Signal(signed(16))
        m.d.comb += shifted.eq(latent)
        with m.If(shifted > T_MAX):
            m.d.comb += t_next.eq(T_MAX)
        with m.Elif(shifted < T_MIN):
            m.d.comb += t_next.eq(T_MIN)
        with m.Else():
            m.d.comb += t_next.eq(shifted)

        # Stage 3, its first clock: output o's row of U, kept. Its second: the sum of the latents
        # by that row, with output o's h and bias. Its third: z exactly, then shifted and clamped.
        u_1 = Signal(unsigned(rank))
        o_1 = Signal.like(o)
        o_2 = Signal.like(o)
        valid_u1 = Signal()
        valid_u2 = Signal()
        s = Signal(range(-s_max, s_max + 1))
        h_2 = Signal(signed(8))
        bias_2 = Signal(signed(32))
        if variant == "A":
            m.d.comb += [u.index.eq(o), h.index.eq(o_1), bias.index.eq(o_1)]
        m.d.sync += [
            valid_u1.eq(0),
            s.eq(signed_sum([Mux(u_1[j], nt[j], t[j]) for j in range(rank)])),
            h_2.eq(h.data),
            bias_2.eq(bias.data),
            o_2.eq(o_1),
            valid_u2.eq(valid_u1),
        ]
        exact = s * h_2 + bias_2
        z_exact = Signal(exact.shape())
        y_shifted = Signal((z_exact >> layer["output_shift"]).shape())
        y_next = Signal(8)
        m.d.comb += [z_exact.eq(exact), y_shifted.eq(z_exact >> layer["output_shift"])]
        with m.If(y_shifted < 0):
            m.d.comb += y_next.eq(0)
        with m.Elif(y_shifted > 255):
            m.d.comb += y_next.eq(255)
        with m.Else():
            m.d.comb += y_next.eq(y_shifted)
        m.d.sync += [
            self.y.eq(y_next),
            self.z.eq(z_exact),
            self.y_valid.eq(valid_u2),
            self.y_last.eq(valid_u2 & (o_2 == n_out - 1)),
        ]

        ready, take = admit(m, variant, self.x_valid)
        with m.FSM() as fsm:
            with m.State("V"):
                m.d.comb += self.x_ready.eq(ready)
                with m.If(take):
                    m.d.sync += [
                        x_1.eq(self.x),
                        g_1.eq(g.data),
                        v_1.eq(v.data),
                        first_1.eq(i == 0),
                        valid_1.eq(1),
                    ]
                    with m.If(i == n_in - 1):
                        m.d.sync += i.eq(0)
                        m.next = "V-last"
                    with m.Else():
                        m.d.sync += i.eq(i + 1)
            # The clock on which stage 1 adds its last input in.
            with m.State("V-last"):
                m.next = "L"
            with m.State("L"):
                for j in range(rank - 1):
                    m.d.sync += [a[j].eq(a[j + 1]), t[j].eq(t[j + 1]), nt[j].eq(nt[j + 1])]
                m.d.sync += [t[rank - 1].eq(t_next), nt[rank - 1].eq(-t_next)]
                with m.If(k == rank - 1):
                    m.d.sync += k.eq(0)
                    m.next = "U"
                with m.Else():
                    m.d.sync += k.eq(k + 1)
            with m.State("U"):
                m.d.sync += [u_1.eq(u.data), o_1.eq(o), valid_u1.eq(1)]
                with m.If(o == n_out - 1):
                    m.d.sync += o.eq(0)
                    m.next = "V"
                with m.Else():
                    m.d.sync += o.eq(o + 1)

        # Variant B's memories, each given the index the engine will be at on the next clock:
        # stage 1's input i, taken on a clock x_valid and x_ready are both high; stage 3's output
        # o, one more each clock in U; and o_1, o a clock behind, which takes o's value in U.
        if variant == "B":
            i_next = following(i, n_in - 1, self.x_valid & self.x_ready)
            o_next = following(o, n_out - 1, fsm.ongoing("U"))
            o_1_next = Mux(fsm.ongoing("U"), o, o_1)
            m.d.comb += [
                g.index.eq(i_next),
                v.index.eq(i_next),
                u.index.eq(o_next),
                h.index.eq(o_1_next),
                bias.index.eq(o_1_next),
            ]
        return m


class DenseEngine(Elaboratable):
    """The output layer, `dense` as integer.json holds it (a dict), applied to each vector of
    uint8s streamed into `x`, then the argmax: what `DenseLayer::scores` and `argmax` compute in
    crates/littlebit/src/integer.rs.

    One input a clock, for `inputs` clocks, and one more: each input and its column of the weights
    are kept for a clock, then every output's accumulator adds the input times its weight, one
    multiplier each, starting from its bias. Then the argmax, one output a clock for `outputs`
    clocks: the first of the highest scores wins, as the golden model's does.

    In: `x`, `x_valid` and `x_ready`, as the LittleBit engine's. Out: `scores`, an int32 for each
    output, and `digit`, the index of the first highest. Both are valid on the clock `done` is
    high, and held until the clock after the next vector's first input is taken. A vector takes
    inputs + 1 + outputs clocks, and the last of them raises `done`.

    The accumulators are as wide as the scores can get, from the weights and biases, so none
    wraps; a layer whose scores could pass an int32 is refused, as the golden model refuses one
    that does. `variant` says where the weights are, as the LittleBit engine's does; the biases
    are the accumulators' starts, and stay logic in both."""

    def __init__(self, dense, variant="A"):
        check_variant(variant)
        self.variant = variant
        self.inputs = dense["inputs"]
        self.outputs = dense["outputs"]
        weight, bias = dense["weight"], dense["bias"]
        if len(weight) != self.outputs or len(bias) != self.outputs:
            raise ValueError("the weight or the bias is not as long as the layer's outputs")
        if any(len(row) != self.inputs for row in weight):
            raise ValueError("a row of the weight is not as long as the layer's inputs")
        # Every partial sum lies between its bias plus the negative weights' share and its bias
        # plus the positive weights', each input at most 255.
        self.low = min(b + 255 * sum(w for w in row if w < 0) for row, b in zip(weight, bias))
        self.high = max(b + 255 * sum(w for w in row if w > 0) for row, b in zip(weight, bias))
        if self.low < -(1 << 31) or self.high > (1 << 31) - 1:
            raise ValueError(f"a score could reach {self.low} or {self.high}, past an int32")
        self.dense = dense
        self.x = Signal(8)
        self.x_valid = Signal()
        self.x_ready = Signal()
        self.scores = [Signal(signed(32), name=f"score_{o}") for o in range(self.outputs)]
        self.digit = Signal(range(max(self.outputs, 2)))
        self.done = Signal()

    def ports(self):
        return [self.x, self.x_valid, self.x_ready, *self.scores, self.digit, self.done]

    def elaborate(self, platform):
        m = Module()
        weight, bias = self.dense["weight"], self.dense["bias"]
        n_in, n_out = self.inputs, self.outputs

        # Input i's column of the weights: output o's int8 in bits 8o to 8o + 7.
        columns = [sum((weight[o][i] & 0xFF) << (8 * o) for o in range(n_out)) for i in range(n_in)]
        m.submodules.w = w = table(columns, unsigned(8 * n_out), self.variant)

        i = Signal(range(max(n_in, 2)))
        j = Signal(range(max(n_out, 2)))
        acc = [Signal(range(self.low, self.high + 1), name=f"acc_{o}") for o in range(n_out)]
        m.d.comb += [score.eq(a) for score, a in zip(self.scores, acc)]

        # The input and its column, kept; a clock later, each product added in.
        x_1 = Signal(8)
        w_1 = Signal(unsigned(8 * n_out))
        first_1 = Signal()
        valid_1 = Signal()
        if self.variant == "A":
            m.d.comb += w.index.eq(i)
        m.d.sync += valid_1.eq(0)
        with m.If(valid_1):
            for o in range(n_out):
                start = Mux(first_1, Const(bias[o], acc[o].shape()), acc[o])
                m.d.sync += acc[o].eq(start + x_1 * w_1.word_select(o, 8).as_signed())

        best = Signal(range(self.low, self.high + 1))
        m.d.sync += self.done.eq(0)
        ready, take = admit(m, self.variant, self.x_valid)
        with m.FSM():
            with m.State("sum"):
                m.d.comb += self.x_ready.eq(ready)
                with m.If(take):
                    m.d.sync += [x_1.eq(self.x), w_1.eq(w.data), first_1.eq(i == 0), valid_1.eq(1)]
                    with m.If(i == n_in - 1):
                        m.d.sync += i.eq(0)
                        m.next = "last"
                    with m.Else():
                        m.d.sync += i.eq(i + 1)
            # The clock on which the last input is added in.
            with m.State("last"):
                m.next = "argmax"
            with m.State("argmax"):
                score = Array(acc)[j]
                with m.If((j == 0) | (score > best)):
                    m.d.sync += [best.eq(score), self.digit.eq(j)]
                with m.If(j == n_out - 1):
                    m.d.sync += [j.eq(0), self.done.eq(1)]
                    m.next = "sum"
                with m.Else():
                    m.d.sync += j.eq(j + 1)

        # In Variant B, the weights' memory is given the input the engine will be at on the next
        # clock, as the LittleBit engine's are.
        if self.variant == "B":
            m.d.comb += w.index.eq(following(i, n_in - 1, self.x_valid & self.x_ready))
        return m


class LittleBitNetwork(Elaboratable):
    """A whole network, `network` as integer.json holds it (a dict), one image at a time: what
    `Network::scores` and `Network::predict` compute in crates/littlebit/src/integer.rs.

    Each pixel is shifted right by the network's pixel shift into the first LittleBit layer's
    engine. Each engine's outputs go straight into the next engine, and the last's into the output
    layer, `DenseEngine`, as they come, one a clock. None of them has to wait for the one before
    to finish, and none can be handed an output it isn't ready for: the network takes an image's
    first pixel only once the image before is done, so each engine is idle when its first input
    of an image comes, and takes all its inputs back to back.

    In: `x`, an image's pixels in order, each taken on a clock when `x_valid` and `x_ready` are
    both high. Out: `scores`, `digit` and `done`, as the output layer's, and `cycles`, a uint32:
    the clocks the image took, from the one that took its first pixel to the one that raised
    `done`, both counted. The next image's first pixel can be taken on the clock `done` is high.
    `variant`, "A" or "B", is every engine's."""

    def __init__(self, network, variant="A"):
        check_variant(variant)
        layers, dense = network["layers"], network["output"]
        inputs = 784
        for j, layer in enumerate(layers):
            if layer["inputs"] != inputs:
                raise ValueError(f"layer {j} takes {layer['inputs']} inputs, not {inputs}")
            inputs = layer["outputs"]
        if dense["inputs"] != inputs:
            raise ValueError(f"the output layer takes {dense['inputs']} inputs, not {inputs}")
        self.pixel_shift = network["pixel_shift"]
        self.engines = [LittleBitEngine(layer, variant) for layer in layers]
        self.dense = DenseEngine(dense, variant)
        self.x = Signal(8)
        self.x_valid = Signal()
        self.x_ready = Signal()
        self.scores = self.dense.scores
        self.digit = self.dense.digit
        self.done = self.dense.done
        self.cycles = Signal(32)

    def ports(self):
        return [
            self.x, self.x_valid, self.x_ready, *self.scores, self.digit, self.done, self.cycles
        ]

    def elaborate(self, platform):
        m = Module()
        for j, engine in enumerate(self.engines):
            m.submodules[f"layer_{j + 1}"] = engine
        m.submodules.dense = dense = self.dense
        first = self.engines[0] if self.engines else dense
        for before, after in zip(self.engines, [*self.engines[1:], dense]):
            m.d.comb += [after.x.eq(before.y), after.x_valid.eq(before.y_valid)]

        # Pixels: from the first to the last of an image, then none until it is done.
        pixel = Signal(range(784))
        waiting = Signal()
        taken = Signal()
        open_ = ~waiting | self.done
        m.d.comb += [
            first.x.eq(self.x >> self.pixel_shift),
            first.x_valid.eq(self.x_valid & open_),
            self.x_ready.eq(first.x_ready & open_),
            taken.eq(self.x_valid & self.x_ready),
        ]
        with m.If(taken):
            with m.If(pixel == 783):
                m.d.sync += [pixel.eq(0), waiting.eq(1)]
            with m.Else():
                m.d.sync += pixel.eq(pixel + 1)
        with m.If(self.done):
            m.d.sync += waiting.eq(0)

        # The clocks from the first pixel taken to `done`, both counted.
        counting = Signal()
        with m.If(taken & (pixel == 0)):
            m.d.sync += [self.cycles.eq(1), counting.eq(1)]
        with m.Elif(counting):
            with m.If(self.done):
                m.d.sync += counting.eq(0)
            with m.Else():
                m.d.sync += self.cycles.eq(self.cycles + 1)
        return m


class Alone(Elaboratable):
    """An engine as its own top, for Yosys and nextpnr to size: its ports, less `z`, which only
    tests read, so that synthesis drops what makes it."""

    def __init__(self, layer, variant="A"):
        self.engine = LittleBitEngine(layer, variant)
        self.x = Signal(8)
        self.x_valid = Signal()
        self.x_ready = Signal()
        self.y = Signal(8)
        self.y_valid = Signal()
        self.y_last = Signal()

    def ports(self):
        return [self.x, self.x_valid, self.x_ready, self.y, self.y_valid, self.y_last]

    def elaborate(self, platform):
        m = Module()
        m.submodules.engine = e = self.engine
        m.d.comb += [
            e.x.eq(self.x),
            e.x_valid.eq(self.x_valid),
            self.x_ready.eq(e.x_ready),
            self.y.eq(e.y),
            self.y_valid.eq(e.y_valid),
            self.y_last.eq(e.y_last),
        ]
        return m


if __name__ == "__main__":
    # python -m gateware.cores.littlebit <integer.json> <layer index> <out.v> [A | B], from the
    # repo root. The engine alone, in Variant A unless B is named, with nothing at its ports:
    # then, for instance,
    #   yosys -p "read_verilog <out.v>; synth_ecp5 -top engine -json <out.json>"
    #   nextpnr-ecp5 --45k --package CABGA381 --speed 6 --freq 50 --seed 1 --json <out.json>
    import json
    import sys
    from pathlib import Path

    from amaranth.back import verilog

    network, index, out = sys.argv[1], int(sys.argv[2]), Path(sys.argv[3])
    variant = sys.argv[4] if len(sys.argv) > 4 else "A"
    top = Alone(json.loads(Path(network).read_text())["layers"][index], variant)
    out.write_text(verilog.convert(top, name="engine", ports=top.ports()))
