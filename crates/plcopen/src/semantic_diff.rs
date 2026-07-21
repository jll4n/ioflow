use std::collections::HashMap;

use serde::Serialize;

use crate::types::*;

/// Résultat du diff sémantique entre deux projets PLCopenXML.
#[derive(Debug, Serialize)]
pub struct PlcDiff {
    pub pous_added: Vec<String>,
    pub pous_removed: Vec<String>,
    pub pous_modified: Vec<PouDiff>,
}

impl PlcDiff {
    pub fn has_changes(&self) -> bool {
        !self.pous_added.is_empty()
            || !self.pous_removed.is_empty()
            || !self.pous_modified.is_empty()
    }
}

/// Diff sur un POU individuel.
#[derive(Debug, Serialize)]
pub struct PouDiff {
    pub name: String,
    pub vars_added: Vec<VarChange>,
    pub vars_removed: Vec<VarChange>,
    pub networks_added: Vec<u32>,
    pub networks_removed: Vec<u32>,
    pub networks_modified: Vec<u32>,
}

impl PouDiff {
    fn has_changes(&self) -> bool {
        !self.vars_added.is_empty()
            || !self.vars_removed.is_empty()
            || !self.networks_added.is_empty()
            || !self.networks_removed.is_empty()
            || !self.networks_modified.is_empty()
    }
}

/// Un changement de variable (ajout ou suppression).
#[derive(Debug, Serialize)]
pub struct VarChange {
    pub name: String,
    pub data_type: String,
}

/// Compare deux projets PLCopen et retourne le diff sémantique.
pub fn diff_projects(a: &Project, b: &Project) -> PlcDiff {
    let a_pous: HashMap<&str, &Pou> = a.pous.iter().map(|p| (p.name.as_str(), p)).collect();
    let b_pous: HashMap<&str, &Pou> = b.pous.iter().map(|p| (p.name.as_str(), p)).collect();

    let mut pous_added: Vec<String> = b_pous
        .keys()
        .filter(|k| !a_pous.contains_key(*k))
        .map(|k| k.to_string())
        .collect();
    pous_added.sort();

    let mut pous_removed: Vec<String> = a_pous
        .keys()
        .filter(|k| !b_pous.contains_key(*k))
        .map(|k| k.to_string())
        .collect();
    pous_removed.sort();

    let mut pous_modified: Vec<PouDiff> = b_pous
        .iter()
        .filter_map(|(name, pou_b)| a_pous.get(name).map(|pou_a| diff_pous(pou_a, pou_b)))
        .filter(|d| d.has_changes())
        .collect();
    pous_modified.sort_by(|a, b| a.name.cmp(&b.name));

    PlcDiff {
        pous_added,
        pous_removed,
        pous_modified,
    }
}

fn diff_pous(a: &Pou, b: &Pou) -> PouDiff {
    let a_vars = collect_vars(&a.interface);
    let b_vars = collect_vars(&b.interface);

    let a_var_names: std::collections::HashSet<&str> =
        a_vars.iter().map(|v| v.name.as_str()).collect();
    let b_var_names: std::collections::HashSet<&str> =
        b_vars.iter().map(|v| v.name.as_str()).collect();

    let vars_added: Vec<VarChange> = b_vars
        .iter()
        .filter(|v| !a_var_names.contains(v.name.as_str()))
        .map(|v| VarChange {
            name: v.name.clone(),
            data_type: format_type(&v.data_type),
        })
        .collect();

    let vars_removed: Vec<VarChange> = a_vars
        .iter()
        .filter(|v| !b_var_names.contains(v.name.as_str()))
        .map(|v| VarChange {
            name: v.name.clone(),
            data_type: format_type(&v.data_type),
        })
        .collect();

    let (networks_added, networks_removed, networks_modified) = match (&a.body, &b.body) {
        (Body::Ld(la), Body::Ld(lb)) => diff_networks(&la.networks, &lb.networks),
        _ => (vec![], vec![], vec![]),
    };

    PouDiff {
        name: b.name.clone(),
        vars_added,
        vars_removed,
        networks_added,
        networks_removed,
        networks_modified,
    }
}

fn collect_vars(iface: &Interface) -> Vec<&Variable> {
    let mut vars = Vec::new();
    vars.extend(iface.input_vars.iter());
    vars.extend(iface.output_vars.iter());
    vars.extend(iface.in_out_vars.iter());
    vars.extend(iface.local_vars.iter());
    vars.extend(iface.temp_vars.iter());
    vars.extend(iface.external_vars.iter());
    vars.extend(iface.global_vars.iter());
    vars
}

fn diff_networks(a_nets: &[LdNetwork], b_nets: &[LdNetwork]) -> (Vec<u32>, Vec<u32>, Vec<u32>) {
    let a_map: HashMap<u32, &LdNetwork> = a_nets.iter().map(|n| (n.local_id, n)).collect();
    let b_map: HashMap<u32, &LdNetwork> = b_nets.iter().map(|n| (n.local_id, n)).collect();

    let mut added: Vec<u32> = b_map
        .keys()
        .filter(|id| !a_map.contains_key(id))
        .copied()
        .collect();

    let mut removed: Vec<u32> = a_map
        .keys()
        .filter(|id| !b_map.contains_key(id))
        .copied()
        .collect();

    let mut modified: Vec<u32> = b_map
        .keys()
        .filter(|id| {
            a_map
                .get(id)
                .map(|na| network_key(na) != network_key(b_map[id]))
                .unwrap_or(false)
        })
        .copied()
        .collect();

    added.sort();
    removed.sort();
    modified.sort();
    (added, removed, modified)
}

fn network_key(net: &LdNetwork) -> String {
    net.elements
        .iter()
        .map(|e| match e {
            LdElement::Contact(c) => format!("C:{}:{}:{:?}", c.variable, c.negated, c.edge),
            LdElement::Coil(c) => {
                format!(
                    "L:{}:{}:{:?}:{:?}",
                    c.variable, c.negated, c.storage, c.edge
                )
            }
            LdElement::Block(b) => format!("B:{}", b.type_name),
            LdElement::LeftPowerRail(_) => "LR".into(),
            LdElement::RightPowerRail(_) => "RR".into(),
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn format_type(dt: &DataTypeRef) -> String {
    match dt {
        DataTypeRef::Bool => "BOOL".into(),
        DataTypeRef::SInt => "SINT".into(),
        DataTypeRef::Int => "INT".into(),
        DataTypeRef::DInt => "DINT".into(),
        DataTypeRef::LInt => "LINT".into(),
        DataTypeRef::USInt => "USINT".into(),
        DataTypeRef::UInt => "UINT".into(),
        DataTypeRef::UDInt => "UDINT".into(),
        DataTypeRef::ULInt => "ULINT".into(),
        DataTypeRef::Real => "REAL".into(),
        DataTypeRef::LReal => "LREAL".into(),
        DataTypeRef::String => "STRING".into(),
        DataTypeRef::WString => "WSTRING".into(),
        DataTypeRef::Byte => "BYTE".into(),
        DataTypeRef::Word => "WORD".into(),
        DataTypeRef::DWord => "DWORD".into(),
        DataTypeRef::LWord => "LWORD".into(),
        DataTypeRef::Time => "TIME".into(),
        DataTypeRef::Date => "DATE".into(),
        DataTypeRef::TimeOfDay => "TOD".into(),
        DataTypeRef::DateAndTime => "DT".into(),
        DataTypeRef::Derived(name) => name.clone(),
        DataTypeRef::Array {
            base_type,
            dimensions,
        } => {
            let dims: Vec<String> = dimensions
                .iter()
                .map(|d| format!("{}..{}", d.lower, d.upper))
                .collect();
            format!("ARRAY[{}] OF {}", dims.join(", "), format_type(base_type))
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_project;

    fn xml_with_vars(pou: &str, vars: &[(&str, &str)]) -> String {
        let var_block: String = vars
            .iter()
            .map(|(name, ty)| {
                format!(
                    "<variable name=\"{name}\"><type><{ty}/></type></variable>",
                    name = name,
                    ty = ty
                )
            })
            .collect();

        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://www.plcopen.org/xml/tc6_0201">
  <fileHeader/><contentHeader name="T"/>
  <pous><pou name="{pou}" pouType="program">
    <interface><localVars>{var_block}</localVars></interface>
    <body><ST><xhtml:p xmlns:xhtml="http://www.w3.org/1999/xhtml"></xhtml:p></ST></body>
  </pou></pous>
</project>"#,
            pou = pou,
        )
    }

    #[test]
    fn var_ajoutee() {
        let a = parse_project(&xml_with_vars("P", &[("X", "BOOL")])).unwrap();
        let b = parse_project(&xml_with_vars("P", &[("X", "BOOL"), ("Y", "INT")])).unwrap();
        let diff = diff_projects(&a, &b);
        assert!(!diff.has_changes() == false);
        let pou = &diff.pous_modified[0];
        assert_eq!(pou.vars_added.len(), 1);
        assert_eq!(pou.vars_added[0].name, "Y");
        assert!(pou.vars_removed.is_empty());
    }

    #[test]
    fn var_supprimee() {
        let a = parse_project(&xml_with_vars("P", &[("X", "BOOL"), ("Y", "INT")])).unwrap();
        let b = parse_project(&xml_with_vars("P", &[("X", "BOOL")])).unwrap();
        let diff = diff_projects(&a, &b);
        let pou = &diff.pous_modified[0];
        assert_eq!(pou.vars_removed.len(), 1);
        assert_eq!(pou.vars_removed[0].name, "Y");
    }

    #[test]
    fn pou_ajoute() {
        let a = parse_project(&xml_with_vars("P", &[])).unwrap();
        let b_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://www.plcopen.org/xml/tc6_0201">
  <fileHeader/><contentHeader name="T"/>
  <pous>
    <pou name="P" pouType="program">
      <interface><localVars/></interface>
      <body><ST><xhtml:p xmlns:xhtml="http://www.w3.org/1999/xhtml"></xhtml:p></ST></body>
    </pou>
    <pou name="Q" pouType="functionBlock">
      <interface><localVars/></interface>
      <body><ST><xhtml:p xmlns:xhtml="http://www.w3.org/1999/xhtml"></xhtml:p></ST></body>
    </pou>
  </pous>
</project>"#
        );
        let b = parse_project(&b_xml).unwrap();
        let diff = diff_projects(&a, &b);
        assert_eq!(diff.pous_added, vec!["Q".to_string()]);
        assert!(diff.pous_removed.is_empty());
    }

    #[test]
    fn aucun_changement() {
        let a = parse_project(&xml_with_vars("P", &[("X", "BOOL")])).unwrap();
        let diff = diff_projects(&a, &a);
        assert!(!diff.has_changes());
    }
}
