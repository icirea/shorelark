use approx::relative_eq;
use rand::seq::SliceRandom;
use rand::{Rng, RngCore};
use std::ops::Index;

pub struct GeneticAlgorithm<S> {
    selection_method: S,
    crossover_method: Box<dyn CrossoverMethod>,
    mutation_method: Box<dyn MutationMethod>,
}

pub struct RouletteWheelSelection;

pub trait Individual {
    fn fitness(&self) -> f32;
    fn chromosome(&self) -> &Chromosome;
    fn create(chromosome: Chromosome) -> Self;
}

pub trait SelectionMethod {
    fn select<'a, I>(&self, rng: &mut dyn RngCore, population: &'a [I]) -> &'a I
    where
        I: Individual;
}

#[derive(Clone, Debug)]
pub struct Chromosome {
    genes: Vec<f32>,
}

pub trait CrossoverMethod {
    fn crossover(
        &self,
        rng: &mut dyn RngCore,
        parent_a: &Chromosome,
        parent_b: &Chromosome,
    ) -> Chromosome;
}
#[derive(Clone, Debug)]
pub struct UniformCrossover;

pub trait MutationMethod {
    fn mutate(&self, rng: &mut dyn RngCore, child: &mut Chromosome);
}

pub struct GaussianMutation {
    /// Probability of changing a gene:
    /// - 0.0 = no genes will be touched
    /// - 1.0 = all genes will be touched
    chance: f32,

    /// Magnitude of that change:
    /// - 0.0 = touched genes will not be modified
    /// - 3.0 = touched genes will be += or -= by at most 3.0
    coeff: f32,
}

impl GaussianMutation {
    pub fn new(chance: f32, coeff: f32) -> Self {
        assert!((0.0..=1.0).contains(&chance));

        Self { chance, coeff }
    }
}

impl MutationMethod for GaussianMutation {
    fn mutate(&self, rng: &mut dyn RngCore, child: &mut Chromosome) {
        for gene in child.iter_mut() {
            let sign = if rng.gen_bool(0.5) { -1.0 } else { 1.0 };

            if rng.gen_bool(self.chance as f64) {
                *gene += sign * self.coeff * rng.r#gen::<f32>();
            }
        }
    }
}

impl CrossoverMethod for UniformCrossover {
    fn crossover(
        &self,
        rng: &mut dyn RngCore,
        parent_a: &Chromosome,
        parent_b: &Chromosome,
    ) -> Chromosome {
        assert_eq!(parent_a.len(), parent_b.len());

        parent_a
            .iter()
            .zip(parent_b.iter())
            .map(|(&a, &b)| if rng.gen_bool(0.5) { a } else { b })
            .collect()
    }
}

impl Chromosome {
    pub fn len(&self) -> usize {
        self.genes.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &f32> {
        self.genes.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut f32> {
        self.genes.iter_mut()
    }
}

impl Index<usize> for Chromosome {
    type Output = f32;

    fn index(&self, index: usize) -> &Self::Output {
        &self.genes[index]
    }
}

impl FromIterator<f32> for Chromosome {
    fn from_iter<T: IntoIterator<Item = f32>>(iter: T) -> Self {
        Self {
            genes: iter.into_iter().collect(),
        }
    }
}

impl IntoIterator for Chromosome {
    type Item = f32;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.genes.into_iter()
    }
}

impl PartialEq for Chromosome {
    fn eq(&self, other: &Self) -> bool {
        approx::relative_eq!(self.genes.as_slice(), other.genes.as_slice())
    }
    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}

impl<S> GeneticAlgorithm<S>
where
    S: SelectionMethod,
{
    pub fn new(
        selection_method: S,
        crossover_method: impl CrossoverMethod + 'static,
        mutation_method: impl MutationMethod + 'static,
    ) -> Self {
        Self {
            selection_method,
            crossover_method: Box::new(crossover_method),
            mutation_method: Box::new(mutation_method),
        }
    }
    pub fn evolve<I, T: RngCore>(&self, rng: &mut T, population: &[I]) -> Vec<I>
    where
        I: Individual,
    {
        assert!(!population.is_empty());

        (0..population.len())
            .map(|_| {
                let parent_a = self.selection_method.select(rng, population).chromosome();
                let parent_b = self.selection_method.select(rng, population).chromosome();
                let mut child = self.crossover_method.crossover(rng, parent_a, parent_b);
                self.mutation_method.mutate(rng, &mut child);

                I::create(child)
            })
            .collect()
    }
}

impl SelectionMethod for RouletteWheelSelection {
    fn select<'a, I>(&self, rng: &mut dyn RngCore, population: &'a [I]) -> &'a I
    where
        I: Individual,
    {
        population
            .choose_weighted(rng, |individual| individual.fitness())
            .expect("population should not be empty")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    use std::collections::BTreeMap;
    use std::iter::FromIterator;

    #[derive(Debug, Clone, PartialEq)]
    enum TestIndividual {
        WithChromosome { chromosome: Chromosome },
        WithFitness { fitness: f32 },
    }

    impl TestIndividual {
        fn new(fitness: f32) -> Self {
            Self::WithFitness { fitness }
        }
    }

    impl Individual for TestIndividual {
        fn fitness(&self) -> f32 {
            match self {
                Self::WithChromosome { chromosome } => chromosome.iter().sum(),
                Self::WithFitness { fitness } => *fitness,
            }
        }

        fn chromosome(&self) -> &Chromosome {
            match self {
                Self::WithChromosome { chromosome } => chromosome,
                Self::WithFitness { .. } => {
                    panic!("Not supported for TestIndividual::WithFitness")
                }
            }
        }

        fn create(chromosome: Chromosome) -> Self {
            Self::WithChromosome { chromosome }
        }
    }

    #[test]
    fn roulette_wheel_selection() {
        let mut rng = ChaCha8Rng::from_seed(Default::default());

        let population = vec![
            TestIndividual::new(2.0),
            TestIndividual::new(1.0),
            TestIndividual::new(4.0),
            TestIndividual::new(3.0),
        ];

        let mut actual_histogram = BTreeMap::new();

        for _ in 0..1000 {
            let fitness = RouletteWheelSelection
                .select(&mut rng, &population)
                .fitness() as i32;

            *actual_histogram.entry(fitness).or_insert(0) += 1;
        }

        let expected_histogram = BTreeMap::from_iter([
            // (fitness, how many times this fitness has been chosen)
            (1, 98),
            (2, 202),
            (3, 278),
            (4, 422),
        ]);

        assert_eq!(actual_histogram, expected_histogram);
    }

    #[test]
    fn uniform_crossover() {
        let mut rng = ChaCha8Rng::from_seed(Default::default());
        let parent_a = (1..=100).map(|n| n as f32).collect();
        let parent_b = (1..=100).map(|n| -n as f32).collect();
        let child = UniformCrossover.crossover(&mut rng, &parent_a, &parent_b);

        let diff_a = child.iter().zip(parent_a).filter(|(c, p)| *c != p).count();
        let diff_b = child.iter().zip(parent_b).filter(|(c, p)| *c != p).count();

        assert_eq!(diff_a, 49);
        assert_eq!(diff_b, 51);
    }

    mod gaussian_mutation {
        use super::*;

        fn actual(chance: f32, coeff: f32) -> Vec<f32> {
            let mut rng = ChaCha8Rng::from_seed(Default::default());
            let mut child = vec![1.0, 2.0, 3.0, 4.0, 5.0].into_iter().collect();

            GaussianMutation::new(chance, coeff).mutate(&mut rng, &mut child);

            child.into_iter().collect()
        }
        mod given_zero_chance {
            use approx::assert_relative_eq;

            fn actual(coeff: f32) -> Vec<f32> {
                super::actual(0.0, coeff)
            }
            mod and_zero_coefficient {
                use super::*;
                #[test]
                fn does_not_change_the_original_chromosome() {
                    let actual = actual(0.0);
                    let expected = vec![1.0, 2.0, 3.0, 4.0, 5.0];

                    assert_relative_eq!(actual.as_slice(), expected.as_slice());
                }
            }
            mod and_non_zero_coefficient {
                use super::*;

                #[test]
                fn does_not_change_the_original_chromosome() {
                    let actual = actual(0.5);
                    let expected = vec![1.0, 2.0, 3.0, 4.0, 5.0];

                    assert_relative_eq!(actual.as_slice(), expected.as_slice());
                }
            }
        }

        mod given_fifty_fifty_chance {
            use super::*;
            mod and_zero_coefficient {
                use super::*;
                use approx::assert_relative_eq;
                #[test]
                fn does_not_change_the_original_chromosome() {
                    let actual = actual(0.5, 0.0);
                    let expected = vec![1.0, 2.0, 3.0, 4.0, 5.0];

                    assert_relative_eq!(actual.as_slice(), expected.as_slice());
                }
            }

            mod and_nonzero_coefficient {
                use super::super::*;
                use approx::assert_relative_eq;

                #[test]
                fn slightly_changes_the_original_chromosome() {
                    let actual = actual(0.5, 1.2);
                    let expected = vec![1.0, 1.4614997, 3.0, 4.383233, 5.0];

                    assert_relative_eq!(actual.as_slice(), expected.as_slice());
                }
            }
        }

        mod given_max_chance {
            use super::*;
            mod and_zero_coefficient {
                use super::*;
                use approx::assert_relative_eq;
                #[test]
                fn does_not_change_the_original_chromosome() {
                    let actual = actual(1.0, 0.0);
                    let expected = vec![1.0, 2.0, 3.0, 4.0, 5.0];

                    assert_relative_eq!(actual.as_slice(), expected.as_slice());
                }
            }

            mod and_nonzero_coefficient {
                use super::*;
                use approx::assert_relative_eq;
                #[test]
                fn entirely_changes_the_original_chromosome() {
                    let actual = actual(1.0, 1.2);
                    let expected = vec![2.0908756, 2.278899, 2.4614997, 3.8812299, 4.1328588];

                    assert_relative_eq!(actual.as_slice(), expected.as_slice());
                }
            }
        }
    }

    #[test]
    fn genetic_algorithm() {
        fn individual(genes: &[f32]) -> TestIndividual {
            TestIndividual::create(genes.iter().cloned().collect())
        }

        let mut rng = ChaCha8Rng::from_seed(Default::default());

        let ga = GeneticAlgorithm::new(
            RouletteWheelSelection,
            UniformCrossover,
            GaussianMutation::new(0.5, 0.5),
        );

        let mut population = vec![
            individual(&[0.0, 0.0, 0.0]),
            individual(&[1.0, 1.0, 1.0]),
            individual(&[1.0, 2.0, 1.0]),
            individual(&[1.0, 2.0, 4.0]),
        ];

        for _ in 0..10 {
            population = ga.evolve(&mut rng, &population);
        }

        let expected_population = vec![
            individual(&[0.4476949, 2.0648358, 4.3058133]),
            individual(&[1.2126867, 1.5538777, 2.886911]),
            individual(&[1.0617678, 2.265739, 4.428764]),
            individual(&[0.95909685, 2.4618788, 4.024733]),
        ];

        assert_eq!(population, expected_population);
    }
}
