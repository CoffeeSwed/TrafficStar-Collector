use rand::Rng;

use crate::randomizer::Lcg128XRandomizer;



pub fn shuffle<T : Clone>(vec : &mut [T], randomizer : &mut Lcg128XRandomizer){
    for index in 0..vec.len(){
        let selected_index = randomizer.gen_range(index..vec.len());
        let old = vec[selected_index].clone();
        vec[selected_index] = vec[index].clone();
        vec[index] = old;
    }
}