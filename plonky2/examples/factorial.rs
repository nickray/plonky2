#![allow(clippy::upper_case_acronyms)]

use anyhow::Result;
use plonky2::field::types::Field;
use plonky2::iop::witness::{PartialWitness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::CircuitConfig;
use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

// changes
use std::sync::{Arc, Mutex};
use plonky2::{hash::hash_types::RichField, iop::target::Target};
use plonky2_field::extension::Extendable;

#[derive(Clone)]
struct SmartTarget<F: RichField + Extendable<D>, const D: usize> {
    pub builder: Arc<Mutex<CircuitBuilder<F, D>>>,
    pub inner: Target,
}

impl<F: RichField + Extendable<D>, const D: usize> SmartTarget<F, D> {
    pub fn into_inner(self) -> Target {
        self.inner
    }

    pub fn new(builder: Arc<Mutex<CircuitBuilder<F, D>>>, inner: Target) -> Self {
        Self { builder, inner }
    }

}

impl<F: RichField + Extendable<D>, const D: usize> core::ops::Mul for SmartTarget<F, D> {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        // assert_eq!(self.builder, other.builder);
        let prod = self.builder.lock().unwrap().mul(self.inner, other.inner);
        Self::new(self.builder.clone(), prod)
    }
}

impl<F: RichField + Extendable<D>, const D: usize> core::ops::Mul<u32> for SmartTarget<F, D> {
    type Output = Self;
    fn mul(self, i: u32) -> Self {
        // assert_eq!(self.builder, other.builder);
        let mut builder = self.builder.lock().unwrap();
        let other = builder.constant(F::from_canonical_u32(i));
        let prod = builder.mul(self.inner, other);
        Self::new(self.builder.clone(), prod)
    }
}

impl<F: RichField + Extendable<D>, const D: usize> core::ops::MulAssign<u32> for SmartTarget<F, D> {
    fn mul_assign(&mut self, i: u32) {
        // assert_eq!(self.builder, other.builder);
        let mut builder = self.builder.lock().unwrap();
        let other = builder.constant(F::from_canonical_u32(i));
        let prod = builder.mul(self.inner, other);
        self.inner = prod;
    }
}

struct Builder<F: RichField + Extendable<D>, const D: usize>(Arc<Mutex<CircuitBuilder<F, D>>>);
impl<F: RichField + Extendable<D>, const D: usize> Builder<F, D> {
    pub fn new(config: CircuitConfig) -> Self {
        Self(Arc::new(Mutex::new(CircuitBuilder::new(config))))
    }

    fn with_target(&self, inner: Target) -> SmartTarget<F, D> {
        SmartTarget {
            builder: self.0.clone(),
            inner,
        }
    }

    // pub fn borrow_builder(&self) -> &mut CircuitBuilder<F, D> {
    //     &mut self.0.lock().unwrap()
    // }

    pub fn virtual_target(&self) -> SmartTarget<F, D> {
        // let lock = self.0.lock().unwrap();
        // let inner = lock.add_virtual_target();
        // self.with_target(self.0.lock().unwrap().add_virtual_target())
        self.with_target(self.0.lock().unwrap().add_virtual_target())
    }

    // pub fn canonical_u32(&self, i: u32) -> SmartTarget<F, D> {
    //     // self.with_target(self.0.lock().unwrap().constant(F::from_canonical_u32(i)))
    //     self.with_target(self.0.lock().unwrap().constant(F::from_canonical_u32(i)))
    // }

    pub fn into_inner(self) -> CircuitBuilder<F, D> {
        Arc::try_unwrap(self.0).ok().unwrap().into_inner().unwrap()
    }
}

/// An example of using Plonky2 to prove a statement of the form
/// "I know n * (n + 1) * ... * (n + 99)".
/// When n == 1, this is proving knowledge of 100!.
fn main() -> Result<()> {
    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;

    let config = CircuitConfig::standard_recursion_config();
    let builder = Builder::<F, D>::new(config);

    // The arithmetic circuit.
    let initial = builder.virtual_target();
    let mut cur_target = initial.clone();
    for i in 2..=100 {
        // let i_target = builder.canonical_u32(i);
        // cur_target = cur_target * i_target;
        cur_target *= i;
    }

    let initial = initial.into_inner();
    let cur_target = cur_target.into_inner();
    let mut builder = builder.into_inner();

    // Public inputs are the initial value (provided below) and the result (which is generated).
    builder.register_public_input(initial);
    builder.register_public_input(cur_target);

    let mut pw = PartialWitness::new();
    pw.set_target(initial, F::ONE);

    let data = builder.build::<C>();
    let proof = data.prove(pw)?;

    println!(
        "Factorial starting at {} is {}",
        proof.public_inputs[0], proof.public_inputs[1]
    );

    data.verify(proof)
}
