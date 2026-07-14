use roxmltree::Node;

use crate::error::ParseError;
use crate::types::*;

// ─── Point d'entrée public ────────────────────────────────────────────────────

pub fn parse_project(xml: &str) -> Result<Project, ParseError> {
    let doc = roxmltree::Document::parse(xml)?;
    let root = doc.root_element();

    let mut file_header = FileHeader::default();
    let mut content_header = ContentHeader::default();
    let mut pous = Vec::new();
    let mut data_types = Vec::new();

    for child in elements(root) {
        match child.tag_name().name() {
            "fileHeader" => file_header = parse_file_header(child),
            "contentHeader" => content_header = parse_content_header(child),
            "pous" => {
                for pou_node in elements(child).filter(|n| tag(n, "pou")) {
                    pous.push(parse_pou(pou_node)?);
                }
            }
            "types" => {
                for dt_node in elements(child).filter(|n| tag(n, "dataTypes")) {
                    for dt in elements(dt_node).filter(|n| tag(n, "dataType")) {
                        if let Some(name) = dt.attribute("name") {
                            data_types.push(UserDataType {
                                name: name.to_string(),
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok(Project {
        file_header,
        content_header,
        pous,
        data_types,
    })
}

// ─── En-têtes ─────────────────────────────────────────────────────────────────

fn parse_file_header(node: Node) -> FileHeader {
    FileHeader {
        company_name: attr_opt(node, "companyName"),
        product_name: attr_opt(node, "productName"),
        product_version: attr_opt(node, "productVersion"),
        creation_date_time: attr_opt(node, "creationDateTime"),
        last_modified_date_time: attr_opt(node, "lastModifiedDateTime"),
    }
}

fn parse_content_header(node: Node) -> ContentHeader {
    let mut header = ContentHeader {
        name: attr_opt(node, "name").unwrap_or_default(),
        version: attr_opt(node, "version"),
        author: None,
        organization: None,
        description: None,
    };
    for child in elements(node) {
        match child.tag_name().name() {
            "author" => header.author = child.text().map(str::to_string),
            "organization" => header.organization = child.text().map(str::to_string),
            "description" => header.description = child.text().map(str::to_string),
            _ => {}
        }
    }
    header
}

// ─── POU ─────────────────────────────────────────────────────────────────────

fn parse_pou(node: Node) -> Result<Pou, ParseError> {
    let name = node
        .attribute("name")
        .ok_or(ParseError::MissingAttr {
            element: "pou",
            attr: "name",
        })?
        .to_string();

    let pou_type = match node.attribute("pouType").ok_or(ParseError::MissingAttr {
        element: "pou",
        attr: "pouType",
    })? {
        "program" => PouType::Program,
        "functionBlock" => PouType::FunctionBlock,
        "function" => PouType::Function,
        other => return Err(ParseError::UnknownPouType(other.to_string())),
    };

    let mut interface = Interface::default();
    let mut body_node = None;

    for child in elements(node) {
        match child.tag_name().name() {
            "interface" => interface = parse_interface(child)?,
            "body" => body_node = Some(child),
            _ => {}
        }
    }

    let body = parse_body(
        body_node.ok_or_else(|| ParseError::NoBody(name.clone()))?,
        &name,
    )?;

    Ok(Pou {
        name,
        pou_type,
        interface,
        body,
    })
}

// ─── Interface ────────────────────────────────────────────────────────────────

fn parse_interface(node: Node) -> Result<Interface, ParseError> {
    let mut iface = Interface::default();
    for child in elements(node) {
        let vars = parse_var_list(child)?;
        match child.tag_name().name() {
            "returnType" => {
                if let Some(type_node) = elements(child).next() {
                    iface.return_type = Some(parse_data_type_ref(type_node)?);
                }
            }
            "inputVars" => iface.input_vars = vars,
            "outputVars" => iface.output_vars = vars,
            "inOutVars" => iface.in_out_vars = vars,
            "localVars" => iface.local_vars = vars,
            "tempVars" => iface.temp_vars = vars,
            "externalVars" => iface.external_vars = vars,
            "globalVars" => iface.global_vars = vars,
            _ => {}
        }
    }
    Ok(iface)
}

fn parse_var_list(node: Node) -> Result<Vec<Variable>, ParseError> {
    elements(node)
        .filter(|n| tag(n, "variable"))
        .map(parse_variable)
        .collect()
}

fn parse_variable(node: Node) -> Result<Variable, ParseError> {
    let name = node
        .attribute("name")
        .ok_or(ParseError::MissingAttr {
            element: "variable",
            attr: "name",
        })?
        .to_string();

    let retain = node
        .attribute("retain")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    let constant = node
        .attribute("constant")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    let mut data_type = DataTypeRef::Bool;
    let mut initial_value = None;
    let mut comment = None;

    for child in elements(node) {
        match child.tag_name().name() {
            "type" => {
                if let Some(type_node) = elements(child).next() {
                    data_type = parse_data_type_ref(type_node)?;
                }
            }
            "initialValue" => initial_value = child.text().map(str::to_string),
            "comment" => comment = child.text().map(str::to_string),
            _ => {}
        }
    }

    Ok(Variable {
        name,
        data_type,
        initial_value,
        comment,
        retain,
        constant,
    })
}

fn parse_data_type_ref(node: Node) -> Result<DataTypeRef, ParseError> {
    Ok(match node.tag_name().name() {
        "BOOL" => DataTypeRef::Bool,
        "SINT" => DataTypeRef::SInt,
        "INT" => DataTypeRef::Int,
        "DINT" => DataTypeRef::DInt,
        "LINT" => DataTypeRef::LInt,
        "USINT" => DataTypeRef::USInt,
        "UINT" => DataTypeRef::UInt,
        "UDINT" => DataTypeRef::UDInt,
        "ULINT" => DataTypeRef::ULInt,
        "REAL" => DataTypeRef::Real,
        "LREAL" => DataTypeRef::LReal,
        "STRING" => DataTypeRef::String,
        "WSTRING" => DataTypeRef::WString,
        "BYTE" => DataTypeRef::Byte,
        "WORD" => DataTypeRef::Word,
        "DWORD" => DataTypeRef::DWord,
        "LWORD" => DataTypeRef::LWord,
        "TIME" => DataTypeRef::Time,
        "DATE" => DataTypeRef::Date,
        "TIME_OF_DAY" | "TOD" => DataTypeRef::TimeOfDay,
        "DATE_AND_TIME" | "DT" => DataTypeRef::DateAndTime,
        "derived" => DataTypeRef::Derived(
            node.attribute("name")
                .ok_or(ParseError::MissingAttr {
                    element: "derived",
                    attr: "name",
                })?
                .to_string(),
        ),
        "array" => {
            let base_type = elements(node)
                .find(|n| tag(n, "baseType"))
                .and_then(|n| elements(n).next())
                .ok_or(ParseError::MissingElement("baseType"))?;

            let dimensions = elements(node)
                .filter(|n| tag(n, "dimension"))
                .map(|d| {
                    Ok(ArrayDimension {
                        lower: d.attribute("lower").unwrap_or("0").parse().unwrap_or(0),
                        upper: d.attribute("upper").unwrap_or("0").parse().unwrap_or(0),
                    })
                })
                .collect::<Result<Vec<_>, ParseError>>()?;

            DataTypeRef::Array {
                base_type: Box::new(parse_data_type_ref(base_type)?),
                dimensions,
            }
        }
        other => return Err(ParseError::UnknownDataType(other.to_string())),
    })
}

// ─── Corps du POU ─────────────────────────────────────────────────────────────

fn parse_body(node: Node, pou_name: &str) -> Result<Body, ParseError> {
    let lang_node = elements(node)
        .next()
        .ok_or_else(|| ParseError::NoBody(pou_name.to_string()))?;

    Ok(match lang_node.tag_name().name() {
        "LD" => Body::Ld(parse_ld(lang_node)?),
        "FBD" => Body::Fbd(parse_fbd(lang_node)?),
        "ST" => Body::St(parse_st(lang_node)),
        "IL" => Body::Il(parse_il(lang_node)),
        "SFC" => Body::Sfc(parse_sfc(lang_node)),
        _ => Body::St(StBody {
            text: lang_node.text().unwrap_or("").to_string(),
        }),
    })
}

// ─── Ladder Diagram ───────────────────────────────────────────────────────────

fn parse_ld(node: Node) -> Result<LdBody, ParseError> {
    let mut networks = Vec::new();
    for network_node in elements(node).filter(|n| tag(n, "network")) {
        networks.push(parse_ld_network(network_node)?);
    }
    Ok(LdBody { networks })
}

fn parse_ld_network(node: Node) -> Result<LdNetwork, ParseError> {
    let local_id = parse_local_id(node, "network")?;
    let mut name = None;
    let mut comment = None;
    let mut elements_vec = Vec::new();

    for child in elements(node) {
        match child.tag_name().name() {
            "name" => name = child.text().map(str::to_string),
            "comment" => comment = child.text().map(str::to_string),
            "leftPowerRail" => elements_vec.push(LdElement::LeftPowerRail(parse_left_rail(child)?)),
            "rightPowerRail" => {
                elements_vec.push(LdElement::RightPowerRail(parse_right_rail(child)?))
            }
            "contact" => elements_vec.push(LdElement::Contact(parse_contact(child)?)),
            "coil" => elements_vec.push(LdElement::Coil(parse_coil(child)?)),
            "block" => elements_vec.push(LdElement::Block(parse_ld_block(child)?)),
            _ => {}
        }
    }

    Ok(LdNetwork {
        local_id,
        name,
        comment,
        elements: elements_vec,
    })
}

fn parse_left_rail(node: Node) -> Result<LeftPowerRail, ParseError> {
    let local_id = parse_local_id(node, "leftPowerRail")?;
    let height = node
        .attribute("height")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);

    let position = parse_position(node);

    let connection_points_out = elements(node)
        .filter(|n| tag(n, "connectionPointOut"))
        .map(parse_cp_out)
        .collect();

    Ok(LeftPowerRail {
        local_id,
        position,
        height,
        connection_points_out,
    })
}

fn parse_right_rail(node: Node) -> Result<RightPowerRail, ParseError> {
    let local_id = parse_local_id(node, "rightPowerRail")?;
    let height = node
        .attribute("height")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);

    let position = parse_position(node);

    let connection_points_in = elements(node)
        .filter(|n| tag(n, "connectionPointIn"))
        .map(parse_cp_in)
        .collect();

    Ok(RightPowerRail {
        local_id,
        position,
        height,
        connection_points_in,
    })
}

fn parse_contact(node: Node) -> Result<Contact, ParseError> {
    let local_id = parse_local_id(node, "contact")?;
    let negated = node
        .attribute("negated")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);
    let edge = parse_edge(node.attribute("edge").unwrap_or("none"));

    let position = parse_position(node);
    let variable = child_text(node, "variable").unwrap_or_default();

    let connection_point_in = elements(node)
        .find(|n| tag(n, "connectionPointIn"))
        .map(parse_cp_in);
    let connection_point_out = elements(node)
        .find(|n| tag(n, "connectionPointOut"))
        .map(parse_cp_out);

    Ok(Contact {
        local_id,
        position,
        negated,
        edge,
        variable,
        connection_point_in,
        connection_point_out,
    })
}

fn parse_coil(node: Node) -> Result<Coil, ParseError> {
    let local_id = parse_local_id(node, "coil")?;
    let negated = node
        .attribute("negated")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);
    let storage = parse_coil_storage(node.attribute("storage").unwrap_or("none"));
    let edge = parse_coil_edge(node.attribute("edge").unwrap_or("none"));

    let position = parse_position(node);
    let variable = child_text(node, "variable").unwrap_or_default();

    let connection_point_in = elements(node)
        .find(|n| tag(n, "connectionPointIn"))
        .map(parse_cp_in);
    let connection_point_out = elements(node)
        .find(|n| tag(n, "connectionPointOut"))
        .map(parse_cp_out);

    Ok(Coil {
        local_id,
        position,
        negated,
        storage,
        edge,
        variable,
        connection_point_in,
        connection_point_out,
    })
}

fn parse_ld_block(node: Node) -> Result<LdBlock, ParseError> {
    let local_id = parse_local_id(node, "block")?;
    let type_name = node
        .attribute("typeName")
        .ok_or(ParseError::MissingAttr {
            element: "block",
            attr: "typeName",
        })?
        .to_string();
    let instance_name = attr_opt(node, "instanceName");
    let execution_order_id = node
        .attribute("executionOrderId")
        .and_then(|v| v.parse().ok());

    let position = parse_position(node);

    let mut inputs = Vec::new();
    let mut outputs = Vec::new();

    for child in elements(node) {
        match child.tag_name().name() {
            "inputVariables" => {
                for var in elements(child).filter(|n| tag(n, "variable")) {
                    inputs.push(parse_block_pin(var));
                }
            }
            "outputVariables" => {
                for var in elements(child).filter(|n| tag(n, "variable")) {
                    outputs.push(parse_block_pin(var));
                }
            }
            _ => {}
        }
    }

    Ok(LdBlock {
        local_id,
        position,
        type_name,
        instance_name,
        execution_order_id,
        inputs,
        outputs,
    })
}

fn parse_block_pin(node: Node) -> BlockPin {
    let formal_parameter = attr_opt(node, "formalParameter").unwrap_or_default();
    let mut rel_position = RelPosition::default();
    let mut connections = Vec::new();

    for child in elements(node) {
        match child.tag_name().name() {
            "relPosition" => {
                rel_position = RelPosition {
                    x: child
                        .attribute("x")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0),
                    y: child
                        .attribute("y")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0),
                };
            }
            "connectionPointIn" | "connectionPointOut" => {
                let cp = parse_cp_in(child);
                rel_position = cp.rel_position.clone();
                connections = cp.connections;
            }
            _ => {}
        }
    }

    BlockPin {
        formal_parameter,
        rel_position,
        connections,
    }
}

// ─── Points de connexion ──────────────────────────────────────────────────────

fn parse_cp_in(node: Node) -> ConnectionPointIn {
    let mut rel_position = RelPosition::default();
    let mut connections = Vec::new();

    for child in elements(node) {
        match child.tag_name().name() {
            "relPosition" => {
                rel_position = parse_rel_position(child);
            }
            "connection" => {
                if let Some(ref_id) = child.attribute("refLocalId").and_then(|v| v.parse().ok()) {
                    connections.push(Connection {
                        ref_local_id: ref_id,
                        formal_parameter: attr_opt(child, "formalParameter"),
                    });
                }
            }
            _ => {}
        }
    }

    ConnectionPointIn {
        rel_position,
        connections,
    }
}

fn parse_cp_out(node: Node) -> ConnectionPointOut {
    let rel_position = elements(node)
        .find(|n| tag(n, "relPosition"))
        .map(parse_rel_position)
        .unwrap_or_default();

    ConnectionPointOut {
        rel_position,
        formal_parameter: attr_opt(node, "formalParameter"),
    }
}

// ─── FBD (stub) ───────────────────────────────────────────────────────────────

fn parse_fbd(node: Node) -> Result<FbdBody, ParseError> {
    let mut networks = Vec::new();
    for net in elements(node).filter(|n| tag(n, "network")) {
        let local_id = parse_local_id(net, "network").unwrap_or(0);
        let comment = child_text(net, "comment");
        let blocks = elements(net)
            .filter(|n| tag(n, "block"))
            .map(parse_ld_block)
            .collect::<Result<Vec<_>, _>>()?;
        networks.push(FbdNetwork {
            local_id,
            comment,
            blocks,
        });
    }
    Ok(FbdBody { networks })
}

// ─── ST ───────────────────────────────────────────────────────────────────────

fn parse_st(node: Node) -> StBody {
    // Control Expert encapsule parfois le code dans <xhtml:p> ou en texte direct.
    let text = elements(node)
        .find(|n| n.tag_name().name() == "p")
        .and_then(|n| n.text())
        .or_else(|| node.text())
        .unwrap_or("")
        .trim()
        .to_string();

    StBody { text }
}

// ─── IL ───────────────────────────────────────────────────────────────────────

fn parse_il(node: Node) -> IlBody {
    IlBody {
        text: node.text().unwrap_or("").trim().to_string(),
    }
}

// ─── SFC ──────────────────────────────────────────────────────────────────────

fn parse_sfc(node: Node) -> SfcBody {
    SfcBody {
        raw_xml: node.document().input_text()[node.range()].to_string(),
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn elements(node: Node) -> impl Iterator<Item = Node> {
    node.children().filter(|n| n.is_element())
}

fn tag(node: &Node, name: &str) -> bool {
    node.tag_name().name() == name
}

fn attr_opt(node: Node, name: &str) -> Option<String> {
    node.attribute(name).map(str::to_string)
}

fn child_text(node: Node, tag_name: &str) -> Option<String> {
    elements(node)
        .find(|n| tag(n, tag_name))
        .and_then(|n| n.text().map(str::to_string))
}

fn parse_local_id(node: Node, elem: &'static str) -> Result<u32, ParseError> {
    node.attribute("localId")
        .ok_or(ParseError::MissingAttr {
            element: elem,
            attr: "localId",
        })?
        .parse()
        .map_err(|_| ParseError::MissingAttr {
            element: elem,
            attr: "localId",
        })
}

fn parse_position(node: Node) -> Position {
    elements(node)
        .find(|n| tag(n, "position"))
        .map(|p| Position {
            x: p.attribute("x").and_then(|v| v.parse().ok()).unwrap_or(0),
            y: p.attribute("y").and_then(|v| v.parse().ok()).unwrap_or(0),
        })
        .unwrap_or_default()
}

fn parse_rel_position(node: Node) -> RelPosition {
    RelPosition {
        x: node
            .attribute("x")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
        y: node
            .attribute("y")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
    }
}

fn parse_edge(s: &str) -> EdgeDetection {
    match s {
        "rising" => EdgeDetection::Rising,
        "falling" => EdgeDetection::Falling,
        "both" => EdgeDetection::Both,
        _ => EdgeDetection::None,
    }
}

fn parse_coil_storage(s: &str) -> CoilStorage {
    match s {
        "set" => CoilStorage::Set,
        "reset" => CoilStorage::Reset,
        "retentive" => CoilStorage::Retentive,
        "nonRetentive" => CoilStorage::NonRetentive,
        _ => CoilStorage::None,
    }
}

fn parse_coil_edge(s: &str) -> CoilEdge {
    match s {
        "rising" => CoilEdge::Rising,
        "falling" => CoilEdge::Falling,
        _ => CoilEdge::None,
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_LD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://www.plcopen.org/xml/tc6_0201">
  <fileHeader companyName="ACME" productName="Control Expert" productVersion="15.0"
              creationDateTime="2026-07-14T10:00:00"/>
  <contentHeader name="CONVOYEUR" version="1.0"/>
  <pous>
    <pou name="PROG_CONVOYEUR" pouType="program">
      <interface>
        <localVars>
          <variable name="CAPTEUR_ARRIVEE">
            <type><BOOL/></type>
          </variable>
          <variable name="DEFAUT_MOTEUR">
            <type><BOOL/></type>
          </variable>
          <variable name="MOTEUR_ON">
            <type><BOOL/></type>
          </variable>
          <variable name="TEMPO1">
            <type><derived name="TON"/></type>
          </variable>
        </localVars>
      </interface>
      <body>
        <LD>
          <network localId="1">
            <name>Démarrage convoyeur</name>
            <leftPowerRail localId="10" height="2">
              <position x="0" y="0"/>
              <connectionPointOut>
                <relPosition x="2" y="1"/>
              </connectionPointOut>
            </leftPowerRail>
            <contact localId="20" negated="false" edge="none">
              <position x="50" y="0"/>
              <connectionPointIn>
                <relPosition x="0" y="1"/>
                <connection refLocalId="10"/>
              </connectionPointIn>
              <connectionPointOut>
                <relPosition x="2" y="1"/>
              </connectionPointOut>
              <variable>CAPTEUR_ARRIVEE</variable>
            </contact>
            <contact localId="21" negated="true" edge="none">
              <position x="110" y="0"/>
              <connectionPointIn>
                <relPosition x="0" y="1"/>
                <connection refLocalId="20"/>
              </connectionPointIn>
              <connectionPointOut>
                <relPosition x="2" y="1"/>
              </connectionPointOut>
              <variable>DEFAUT_MOTEUR</variable>
            </contact>
            <coil localId="30" negated="false" storage="none" edge="none">
              <position x="200" y="0"/>
              <connectionPointIn>
                <relPosition x="0" y="1"/>
                <connection refLocalId="21"/>
              </connectionPointIn>
              <variable>MOTEUR_ON</variable>
            </coil>
            <rightPowerRail localId="40" height="2">
              <position x="250" y="0"/>
              <connectionPointIn>
                <relPosition x="0" y="1"/>
                <connection refLocalId="30"/>
              </connectionPointIn>
            </rightPowerRail>
          </network>
        </LD>
      </body>
    </pou>
  </pous>
</project>"#;

    #[test]
    fn parse_projet_ld_basique() {
        let project = parse_project(FIXTURE_LD).expect("Le parsing doit réussir");

        assert_eq!(project.content_header.name, "CONVOYEUR");
        assert_eq!(project.file_header.company_name.as_deref(), Some("ACME"));
        assert_eq!(project.pous.len(), 1);

        let pou = &project.pous[0];
        assert_eq!(pou.name, "PROG_CONVOYEUR");
        assert_eq!(pou.pou_type, PouType::Program);
        assert_eq!(pou.interface.local_vars.len(), 4);

        let Body::Ld(ld) = &pou.body else {
            panic!("Le corps doit être LD");
        };
        assert_eq!(ld.networks.len(), 1);

        let net = &ld.networks[0];
        assert_eq!(net.local_id, 1);
        assert_eq!(net.name.as_deref(), Some("Démarrage convoyeur"));
        assert_eq!(net.elements.len(), 5);

        // Vérification des éléments dans l'ordre
        assert!(matches!(net.elements[0], LdElement::LeftPowerRail(_)));

        let LdElement::Contact(contact1) = &net.elements[1] else {
            panic!("Élément 1 doit être un Contact");
        };
        assert_eq!(contact1.variable, "CAPTEUR_ARRIVEE");
        assert!(!contact1.negated);
        assert_eq!(contact1.local_id, 20);

        let LdElement::Contact(contact2) = &net.elements[2] else {
            panic!("Élément 2 doit être un Contact NF");
        };
        assert_eq!(contact2.variable, "DEFAUT_MOTEUR");
        assert!(contact2.negated, "Contact NF doit être negated=true");

        let LdElement::Coil(coil) = &net.elements[3] else {
            panic!("Élément 3 doit être une Coil");
        };
        assert_eq!(coil.variable, "MOTEUR_ON");
        assert_eq!(coil.storage, CoilStorage::None);

        assert!(matches!(net.elements[4], LdElement::RightPowerRail(_)));
    }

    #[test]
    fn parse_variable_bool() {
        let project = parse_project(FIXTURE_LD).unwrap();
        let var = &project.pous[0].interface.local_vars[0];
        assert_eq!(var.name, "CAPTEUR_ARRIVEE");
        assert_eq!(var.data_type, DataTypeRef::Bool);
    }

    #[test]
    fn parse_variable_type_derive() {
        let project = parse_project(FIXTURE_LD).unwrap();
        let var = &project.pous[0].interface.local_vars[3];
        assert_eq!(var.name, "TEMPO1");
        assert_eq!(var.data_type, DataTypeRef::Derived("TON".to_string()));
    }

    #[test]
    fn connexions_correctes() {
        let project = parse_project(FIXTURE_LD).unwrap();
        let Body::Ld(ld) = &project.pous[0].body else {
            panic!()
        };
        let LdElement::Contact(c) = &ld.networks[0].elements[1] else {
            panic!()
        };

        let cp_in = c.connection_point_in.as_ref().unwrap();
        assert_eq!(cp_in.connections.len(), 1);
        assert_eq!(cp_in.connections[0].ref_local_id, 10);
    }
}
