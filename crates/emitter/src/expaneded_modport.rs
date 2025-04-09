use crate::emitter::{SymbolContext, resolve_generic_path, symbol_string};
use veryl_analyzer::attribute::ExpandItem;
use veryl_analyzer::attribute_table;
use veryl_analyzer::evaluator::Evaluator;
use veryl_analyzer::namespace::Namespace;
use veryl_analyzer::symbol::Direction as SymDirection;
use veryl_analyzer::symbol::Type as SymType;
use veryl_analyzer::symbol::{GenericMap, Port, Symbol, SymbolKind, VariableProperty};
use veryl_analyzer::symbol_table;
use veryl_parser::resource_table::StrId;
use veryl_parser::veryl_grammar_trait::*;
use veryl_parser::veryl_token::{Token, VerylToken};

pub struct ExpandModportConnection {
    pub port_target: VerylToken,
    pub interface_target: VerylToken,
}

pub struct ExpandModportConnections {
    pub connections: Vec<ExpandModportConnection>,
}

impl ExpandModportConnections {
    fn new(
        port: &Port,
        modport: &Symbol,
        interface_name: &VerylToken,
        array_index: &[isize],
    ) -> Self {
        let connections: Vec<_> = collect_modport_member_variables(modport)
            .iter()
            .map(|(variable_token, _variable, _direction)| {
                let (port_target, interface_target) = if array_index.is_empty() {
                    (
                        format!("__{}_{}", port.name(), variable_token),
                        format!("{}.{}", interface_name, variable_token),
                    )
                } else {
                    let index: Vec<_> = array_index.iter().map(|x| format!("{x}")).collect();
                    let select: Vec<_> = array_index.iter().map(|x| format!("[{x}]")).collect();
                    (
                        format!("__{}_{}_{}", port.name(), index.join("_"), variable_token),
                        format!("{}{}.{}", interface_name, select.join(""), variable_token),
                    )
                };
                ExpandModportConnection {
                    port_target: port.token.replace(&port_target),
                    interface_target: interface_name.replace(&interface_target),
                }
            })
            .collect();
        Self { connections }
    }
}

pub struct ExpandModportConnectionsTableEntry {
    id: StrId,
    pub connections: Vec<ExpandModportConnections>,
}

pub struct ExpandModportConnectionsTable {
    entries: Vec<ExpandModportConnectionsTableEntry>,
}

impl ExpandModportConnectionsTable {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn create(
        defined_ports: &[Port],
        connected_ports: &[InstPortItem],
        generic_map: &[GenericMap],
        namespace: &Namespace,
    ) -> Self {
        let mut ret = ExpandModportConnectionsTable::new();
        ret.expand(defined_ports, connected_ports, generic_map, namespace);
        ret
    }

    fn expand(
        &mut self,
        defined_ports: &[Port],
        connected_ports: &[InstPortItem],
        generic_map: &[GenericMap],
        namespace: &Namespace,
    ) {
        for (modport, port) in collect_modports(defined_ports, namespace) {
            if !attribute_table::is_expand(&port.token.token, ExpandItem::Modport) {
                continue;
            }

            let connected_interface = connected_ports
                .iter()
                .find(|x| x.identifier.identifier_token.token.text == port.name())
                .map(|x| {
                    if let Some(ref x) = x.inst_port_item_opt {
                        x.expression.unwrap_identifier().unwrap().identifier()
                    } else {
                        &port.token
                    }
                })
                .unwrap();
            let property = port.property();
            let array_size = evaluate_array_size(&property.r#type.array, generic_map);
            let array_index = expand_array_index(&array_size, &[]);
            let connections: Vec<_> = if array_index.is_empty() {
                vec![ExpandModportConnections::new(
                    &port,
                    &modport,
                    connected_interface,
                    &[],
                )]
            } else {
                array_index
                    .iter()
                    .map(|index| {
                        ExpandModportConnections::new(&port, &modport, connected_interface, index)
                    })
                    .collect()
            };

            let entry = ExpandModportConnectionsTableEntry {
                id: port.name(),
                connections,
            };
            self.entries.push(entry);
        }
    }

    pub fn remove(&mut self, token: &VerylToken) -> Option<ExpandModportConnectionsTableEntry> {
        let index = self.entries.iter().position(|x| x.id == token.token.text)?;
        Some(self.entries.remove(index))
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Clone, Debug)]
pub struct ExpandedModportPort {
    pub identifier: VerylToken,
    pub r#type: SymType,
    pub interface_target: VerylToken,
    pub direction: SymDirection,
    pub direction_token: VerylToken,
}

#[derive(Clone, Debug)]
pub struct ExpandedModportPorts {
    pub ports: Vec<ExpandedModportPort>,
}

impl ExpandedModportPorts {
    fn new(port: &Port, modport: &Symbol, array_index: &[isize]) -> Self {
        let ports: Vec<_> = collect_modport_member_variables(modport)
            .iter()
            .map(|(variable_token, variable, direction)| {
                let (port_name, interface_target) = if array_index.is_empty() {
                    (
                        format!("__{}_{}", port.name(), variable_token),
                        format!("{}.{}", port.name(), variable_token),
                    )
                } else {
                    let index: Vec<_> = array_index.iter().map(|x| format!("{x}")).collect();
                    let select: Vec<_> = array_index.iter().map(|x| format!("[{x}]")).collect();
                    (
                        format!("__{}_{}_{}", port.name(), index.join("_"), variable_token),
                        format!("{}{}.{}", port.name(), select.join(""), variable_token),
                    )
                };
                let direction_token = if matches!(direction, SymDirection::Input) {
                    port.token.replace("input")
                } else {
                    port.token.replace("output")
                };
                ExpandedModportPort {
                    identifier: port.token.replace(&port_name),
                    r#type: variable.r#type.clone(),
                    interface_target: port.token.replace(&interface_target),
                    direction: *direction,
                    direction_token,
                }
            })
            .collect();
        Self { ports }
    }
}

#[derive(Clone, Debug)]
pub struct ExpandedModportPortTableEntry {
    id: StrId,
    pub identifier: VerylToken,
    pub interface_name: VerylToken,
    pub array_size: Vec<isize>,
    pub generic_maps: Vec<GenericMap>,
    pub ports: Vec<ExpandedModportPorts>,
}

pub struct ExpandedModportPortTable {
    entries: Vec<ExpandedModportPortTableEntry>,
}

impl ExpandedModportPortTable {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn create(
        defined_ports: &[Port],
        generic_map: &[GenericMap],
        namespace_token: &VerylToken,
        namespace: &Namespace,
        context: &SymbolContext,
    ) -> Self {
        let mut ret = ExpandedModportPortTable::new();
        ret.expand(
            defined_ports,
            generic_map,
            namespace_token,
            namespace,
            context,
        );
        ret
    }

    fn expand(
        &mut self,
        defined_ports: &[Port],
        generic_map: &[GenericMap],
        namespace_token: &VerylToken,
        namespace: &Namespace,
        context: &SymbolContext,
    ) {
        for (modport, port) in collect_modports(defined_ports, namespace) {
            if !attribute_table::is_expand(&port.token.token, ExpandItem::Modport) {
                continue;
            }

            let Some(interface_symbol) = resolve_interface(&port, namespace, generic_map) else {
                unreachable!()
            };

            let property = port.property();
            let array_size = evaluate_array_size(&property.r#type.array, generic_map);
            let array_index = expand_array_index(&array_size, &[]);
            let interface_name = {
                let text = symbol_string(namespace_token, &interface_symbol, context);
                port.token.replace(&text)
            };

            let ports = if array_index.is_empty() {
                vec![ExpandedModportPorts::new(&port, &modport, &[])]
            } else {
                array_index
                    .iter()
                    .map(|index| ExpandedModportPorts::new(&port, &modport, index))
                    .collect()
            };

            let entry = ExpandedModportPortTableEntry {
                id: port.name(),
                identifier: port.token.clone(),
                interface_name,
                generic_maps: interface_symbol.generic_maps(),
                array_size,
                ports,
            };
            self.entries.push(entry);
        }
    }

    pub fn get(&self, token: &VerylToken) -> Option<ExpandedModportPortTableEntry> {
        self.entries
            .iter()
            .find(|x| x.id == token.token.text)
            .cloned()
    }

    pub fn drain(&mut self) -> Vec<ExpandedModportPortTableEntry> {
        self.entries.drain(..).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn collect_modports(ports: &[Port], namespace: &Namespace) -> Vec<(Symbol, Port)> {
    ports
        .iter()
        .filter_map(|port| {
            let property = port.property();
            if let Some((_, Some(symbol))) = property.r#type.trace_user_defined(namespace) {
                if matches!(symbol.kind, SymbolKind::Modport(_)) {
                    Some((symbol, port.clone()))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

fn evaluate_array_size(array_size: &[Expression], generic_map: &[GenericMap]) -> Vec<isize> {
    let mut evaluator = Evaluator::new(generic_map);
    array_size
        .iter()
        .filter_map(|x| evaluator.expression(x).get_value())
        .collect()
}

fn expand_array_index(array_size: &[isize], array_index: &[Vec<isize>]) -> Vec<Vec<isize>> {
    if array_size.is_empty() {
        return array_index.to_vec();
    }

    let mut array_size = array_size.to_owned();
    let size = array_size.pop().unwrap();

    let mut ret: Vec<_> = Vec::new();
    for s in 0..size {
        if array_index.is_empty() {
            ret.push(vec![s]);
        } else {
            let mut index: Vec<_> = array_index
                .iter()
                .map(|x| {
                    let mut x = x.clone();
                    x.insert(0, s);
                    x
                })
                .collect();
            ret.append(&mut index);
        }
    }

    if array_size.is_empty() {
        ret
    } else {
        expand_array_index(&array_size, &ret)
    }
}

fn collect_modport_member_variables(
    symbol: &Symbol,
) -> Vec<(Token, VariableProperty, SymDirection)> {
    let SymbolKind::Modport(modport) = &symbol.kind else {
        unreachable!()
    };

    modport
        .members
        .iter()
        .filter_map(|member| {
            if let SymbolKind::ModportVariableMember(member) =
                symbol_table::get(*member).unwrap().kind
            {
                let variable_symbol = symbol_table::get(member.variable).unwrap();
                if let SymbolKind::Variable(variable) = variable_symbol.kind {
                    Some((variable_symbol.token, variable, member.direction))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

fn resolve_interface(
    port: &Port,
    namespace: &Namespace,
    generic_map: &[GenericMap],
) -> Option<Symbol> {
    let property = port.property();
    let (user_defined, _) = property.r#type.trace_user_defined(namespace)?;

    let mut path = user_defined.get_user_defined()?.path.clone();
    path.paths.pop(); // remove modport path

    let (result, _) = resolve_generic_path(&path, namespace, Some(&generic_map.to_vec()));
    result.ok().map(|x| x.found)
}
