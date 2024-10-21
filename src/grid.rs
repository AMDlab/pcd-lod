#[derive(Debug)]
pub struct Grid<'a, P> {
    representative: Option<P>,
    candidates: Vec<&'a P>,
}

impl<'a, P> Default for Grid<'a, P> {
    fn default() -> Self {
        Self {
            representative: None,
            candidates: vec![],
        }
    }
}

impl<'a, P> Grid<'a, P> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, representative: P) {
        self.representative = Some(representative);
    }

    pub fn visited(&self) -> bool {
        self.representative.is_some()
    }

    pub fn representative(&self) -> Option<&P> {
        self.representative.as_ref()
    }

    pub fn insert(&mut self, point: &'a P) {
        self.candidates.push(point);
    }

    pub fn candidates(&self) -> &Vec<&'a P> {
        &self.candidates
    }

    pub fn candidates_mut(&mut self) -> &mut Vec<&'a P> {
        &mut self.candidates
    }
}
