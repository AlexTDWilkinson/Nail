#[derive(Debug, Clone)]
pub struct Scope {
    variables: HashMap<String, NailDataTypeDescriptor>,
    parent: Option<Box<Scope>>,
}

impl Scope {
    fn new() -> Self {
        Scope { variables: HashMap::new(), parent: None }
    }

    fn with_parent(parent: Scope) -> Self {
        Scope { variables: HashMap::new(), parent: Some(Box::new(parent)) }
    }

    fn declare(&mut self, name: String, data_type: NailDataTypeDescriptor) {
        self.variables.insert(name, data_type);
    }

    fn get(&self, name: &str) -> Option<&NailDataTypeDescriptor> {
        if let Some(data_type) = self.variables.get(name) {
            Some(data_type)
        } else if let Some(parent) = &self.parent {
            parent.get(name)
        } else {
            None
        }
    }
}

fn enter_scope(state: &mut ParserState) {
    let new_scope = Scope::with_parent(state.current_scope.clone());
    state.current_scope = new_scope;
}

fn exit_scope(state: &mut ParserState) {
    if let Some(parent) = state.current_scope.parent.take() {
        state.current_scope = *parent;
    }
}
