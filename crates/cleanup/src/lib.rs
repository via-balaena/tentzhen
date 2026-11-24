//! Project #3's golden model of SocialJax's Clean Up (`docs/littlebit.md`, "After MNIST: Clean
//! Up's policy"): what the game and its policy compute, step for step, for the gateware to be
//! checked against, as `tentzhen-littlebit` is for MNIST. To match SocialJax exactly it has to draw
//! JAX's random numbers exactly, so that is where it starts: [`random`].

pub mod random;
