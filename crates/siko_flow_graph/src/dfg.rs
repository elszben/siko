use siko_ir::expr::ExprId;
use siko_ir::pattern::PatternId;
use siko_util::dot::Graph;
use siko_util::Counter;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct ValueId {
    pub id: usize,
}

impl fmt::Display for ValueId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{}", self.id)
    }
}

impl From<usize> for ValueId {
    fn from(id: usize) -> ValueId {
        ValueId { id: id }
    }
}

pub enum Edge {
    If(bool),
    Case(usize),
    Jump,
    FnArg(usize),
    ListElement(usize),
    RecordField(usize),
    RecordUpdateSource,
}

pub enum ValueSource {
    Expr(ExprId),
    Arg(usize),
    Pattern(PatternId),
}

pub struct Value {
    id: ValueId,
    source: ValueSource,
    uses: Vec<ExprId>,
}

impl Value {
    pub fn new(id: ValueId, source: ValueSource) -> Value {
        Value {
            id: id,
            source: source,
            uses: Vec::new(),
        }
    }
}

pub struct DataflowGraph {
    next_value: Counter,
    name: String,
    values: BTreeMap<ValueId, Value>,
    edges: Vec<(ValueId, ValueId, Edge)>,
}

impl DataflowGraph {
    pub fn new(name: String) -> DataflowGraph {
        DataflowGraph {
            next_value: Counter::new(),
            name: name,
            values: BTreeMap::new(),
            edges: Vec::new(),
        }
    }

    pub fn create_value(&mut self, source: ValueSource) -> ValueId {
        let value_id = self.next_value.next().into();
        let value = Value::new(value_id, source);
        self.values.insert(value_id, value);
        value_id
    }

    pub fn add_edge(&mut self, from: ValueId, to: ValueId, edge: Edge) {
        self.edges.push((from, to, edge));
    }

    pub fn to_dot_graph(&self) -> Graph {
        let mut graph = Graph::new(self.name.clone());
        let mut index_map = BTreeMap::new();
        for (id, value) in &self.values {
            let value_name = format!("Value{}", id);
            let value_index = graph.add_node(value_name.clone());
            index_map.insert(id, value_index);
        }
        for (from, to, edge) in &self.edges {
            let s = match edge {
                Edge::If(b) => Some(format!("If/{}", b)),
                Edge::Case(i) => Some(format!("Case/{}", i)),
                Edge::FnArg(i) => Some(format!("FnArg/{}", i)),
                Edge::ListElement(i) => Some(format!("ListElement/{}", i)),
                Edge::RecordField(i) => Some(format!("RecordField/{}", i)),
                Edge::RecordUpdateSource => Some(format!("RecordUpdateSource")),
                Edge::Jump => None,
            };
            let to_index = index_map.get(to).expect("To index not found");
            let source_index = index_map.get(from).expect("Source index not found");
            graph.add_edge(s, *source_index, *to_index);
        }
        graph
    }
}
