use rand::{Rng, RngCore};

#[derive(Debug)]
pub struct Network {
    layers: Vec<Layer>,
}

#[derive(Debug, Clone)]
struct Layer {
    neurons: Vec<Neuron>,
}

#[derive(Debug, Clone)]
struct Neuron {
    bias: f32,
    weights: Vec<f32>,
}

pub struct LayerTopology {
    pub neurons: usize,
}

impl Network {
    pub fn propagate(&self, inputs: Vec<f32>) -> Vec<f32> {
        self.layers
            .iter()
            .fold(inputs, |inputs, layer| layer.propagate(inputs))
    }

    pub fn random(rng: &mut dyn RngCore, layers: &[LayerTopology]) -> Self {
        assert!(layers.len() > 1);

        let layers = layers
        .windows(2)
        .map(|layers| Layer::random(rng, layers[0].neurons, layers[1].neurons))
        .collect();

        Self { layers }
    }
}

impl Layer {
    fn propagate(&self, inputs: Vec<f32>) -> Vec<f32> {
        self.neurons
            .iter()
            .map(|neuron| neuron.propagate(&inputs))
            .collect()
    }

    fn random(rng: &mut dyn RngCore, input_size: usize, output_size: usize) -> Self {
        let neurons = (0..output_size)
        .map(|_| Neuron::random(rng, input_size))
        .collect();

        Self { neurons }
    }
}

impl Neuron {
    fn propagate(&self, inputs: &[f32]) -> f32 {
        assert_eq!(inputs.len(), self.weights.len());

        let output = inputs
            .iter()
            .zip(&self.weights)
            .map(|(input, weight)| input * weight)
            .sum::<f32>();

        (self.bias + output).max(0.0)
    }

    fn random(rng: &mut dyn RngCore, input_size: usize) -> Self {
        let bias = rng.gen_range(-1.0..=1.0);

        let weights = (0..input_size)
        .map(|_| rng.gen_range(-1.0..=1.0))
        .collect();

        Self { bias, weights }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn random() {
        // Because we always use the same seed, our `rng` in here will
        // always return the same set of values
        let mut rng = ChaCha8Rng::from_seed(Default::default());
        let neuron = Neuron::random(&mut rng, 4);

        assert_relative_eq!(neuron.bias, -0.6255188);
        assert_relative_eq!(
            neuron.weights.as_slice(),
            [0.67383957, 0.8181262, 0.26284897, 0.5238807].as_ref()
        );
    }

    #[test]
    fn propagate() {
        let neuron = Neuron {
            bias : 0.5,
            weights: vec![-0.3, 0.8],
        };

        // Ensure `max()` (our ReLU) works
        assert_relative_eq!(neuron.propagate(&[-10.0, -10.0]), 0.0);
        assert_relative_eq!(neuron.propagate(&[0.5, 1.0]), (-0.3 * 0.5) + (0.8 * 1.0) + 0.5);

        let layer = Layer {
            neurons: vec![neuron.clone(), neuron.clone()],
        };

        let inputs = vec![0.5, 1.0];
        let propagate_output = layer.propagate(inputs.clone());
        assert_relative_eq!(propagate_output.as_slice(), [1.15, 1.15].as_ref());

        let network = Network {
            layers: vec![layer.clone(), layer.clone()],
        };

        let network_output = network.propagate(inputs.clone());
        assert_relative_eq!(network_output.as_slice(), [1.075, 1.075].as_ref());
    }
}
