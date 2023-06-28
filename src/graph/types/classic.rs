use crate::graph::{Graph, Mutable};

pub struct Classic<V, E> {
    vertices: Vec<V>,
    edges: Vec<E>,
}

pub trait Edge {
    type V;

    fn from(&self) -> Self::V;
    fn to(&self) -> Self::V;
}

impl<V, E> Classic<V, E> {
    pub fn new(vertices: Vec<V>, edges: Vec<E>) -> Self {
        Self { vertices, edges }
    }
}

impl<V, E> Graph for Classic<V, E>
where
    V: PartialEq,
    E: Edge<V = V> + Clone,
{
    type V = V;
    type E = E;

    fn out_vertices(&self, v: &Self::V) -> Vec<Self::V> {
        self.edges
            .iter()
            .filter(|e| &e.from() == v)
            .map(|e| e.to())
            .collect()
    }

    fn in_vertices(&self, v: &Self::V) -> Vec<Self::V> {
        self.edges
            .iter()
            .filter(|e| &e.to() == v)
            .map(|e| e.from())
            .collect()
    }

    fn out_edges(&self, v: &Self::V) -> Vec<Self::E> {
        self.edges
            .iter()
            .filter(|e| &e.from() == v)
            .cloned()
            .collect()
    }

    fn in_edges(&self, v: &Self::V) -> Vec<Self::E> {
        self.edges
            .iter()
            .filter(|e| &e.to() == v)
            .cloned()
            .collect()
    }

    fn edges(&self, from: &Self::V, to: &Self::V) -> Vec<Self::E> {
        self.edges
            .iter()
            .filter(|e| &e.from() == from && &e.to() == to)
            .cloned()
            .collect()
    }

    fn vertices(&self, e: &Self::E) -> (Self::V, Self::V) {
        (e.from(), e.to())
    }
}

impl<V, E> Mutable for Classic<V, E> {
    type V = V;
    type E = E;

    fn add_vertex(&mut self, v: Self::V) {
        self.vertices.push(v);
    }

    fn add_edge(&mut self, e: Self::E) {
        self.edges.push(e);
    }
}
