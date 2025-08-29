use rand::prelude::*;

pub fn generate_mines_vec() -> Vec<u8> {
    let mut positions: Vec<(u8, u8)> = (0..16)
        .flat_map(|x| (0..16).map(move |y| (x as u8, y as u8)))
        .collect();

    let mut rng = rand::rng();
    positions.shuffle(&mut rng);

    positions
        .into_iter()
        .take(40)
        .flat_map(|(x, y)| vec![x, y])
        .collect()
}
