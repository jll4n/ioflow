use std::collections::{HashMap, HashSet};

use crate::types::*;

use super::{element_local_id, svg::render_network_colored, ElemColor};

/// Compare deux réseaux ladder et rend un SVG avec mise en couleur :
/// - vert  = élément ajouté dans `b` (absent de `a`)
/// - rouge = élément supprimé de `a` (absent de `b`)
/// - jaune = élément modifié (même `localId`, contenu différent)
/// - noir  = inchangé
///
/// Le rendu est basé sur les positions de `b` ; les éléments supprimés
/// conservent la position qu'ils avaient dans `a`.
pub fn render_diff(a: &LdNetwork, b: &LdNetwork) -> String {
    // Clés de contenu sémantique (indépendantes des positions)
    let a_keys: HashMap<u32, String> = a
        .elements
        .iter()
        .map(|e| (element_local_id(e), content_key(e)))
        .collect();
    let b_keys: HashMap<u32, String> = b
        .elements
        .iter()
        .map(|e| (element_local_id(e), content_key(e)))
        .collect();

    let b_ids: HashSet<u32> = b_keys.keys().copied().collect();

    // Réseau fusionné : éléments de B + éléments supprimés de A (position A)
    let mut elements = b.elements.clone();
    for elem in &a.elements {
        if !b_ids.contains(&element_local_id(elem)) {
            elements.push(elem.clone());
        }
    }

    let merged = LdNetwork {
        local_id: b.local_id,
        name: b.name.clone().or_else(|| a.name.clone()),
        comment: b.comment.clone(),
        elements,
    };

    render_network_colored(&merged, &|id| match (a_keys.get(&id), b_keys.get(&id)) {
        (None, Some(_)) => ElemColor::Added,
        (Some(_), None) => ElemColor::Removed,
        (Some(ak), Some(bk)) if ak == bk => ElemColor::Normal,
        (Some(_), Some(_)) => ElemColor::Modified,
        (None, None) => ElemColor::Normal,
    })
}

/// Clé de contenu sémantique d'un élément (ignorée : position, connexions).
fn content_key(elem: &LdElement) -> String {
    match elem {
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
    }
}

/// Rendu deux colonnes : SVG de A (éléments supprimés en rouge/modifiés en jaune)
/// et SVG de B (éléments ajoutés en vert/modifiés en jaune).
pub fn render_diff_columns(a: &LdNetwork, b: &LdNetwork) -> (String, String) {
    let a_keys: HashMap<u32, String> = a
        .elements
        .iter()
        .map(|e| (element_local_id(e), content_key(e)))
        .collect();
    let b_keys: HashMap<u32, String> = b
        .elements
        .iter()
        .map(|e| (element_local_id(e), content_key(e)))
        .collect();

    let svg_a = render_network_colored(a, &|id| match (a_keys.get(&id), b_keys.get(&id)) {
        (Some(_), None) => ElemColor::Removed,
        (Some(ak), Some(bk)) if ak != bk => ElemColor::Modified,
        _ => ElemColor::Normal,
    });

    let svg_b = render_network_colored(b, &|id| match (a_keys.get(&id), b_keys.get(&id)) {
        (None, Some(_)) => ElemColor::Added,
        (Some(ak), Some(bk)) if ak != bk => ElemColor::Modified,
        _ => ElemColor::Normal,
    });

    (svg_a, svg_b)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_project;

    fn make_network(var_a: &str, var_b: Option<&str>, negated_a: bool) -> LdNetwork {
        let extra = var_b
            .map(|v| {
                format!(
                    r#"<contact localId="22" negated="false" edge="none">
              <position x="110" y="0"/>
              <connectionPointIn>
                <relPosition x="0" y="1"/>
                <connection refLocalId="21"/>
              </connectionPointIn>
              <connectionPointOut><relPosition x="2" y="1"/></connectionPointOut>
              <variable>{v}</variable>
            </contact>"#
                )
            })
            .unwrap_or_default();

        let coil_ref = if var_b.is_some() { 22 } else { 21 };

        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://www.plcopen.org/xml/tc6_0201">
  <fileHeader/><contentHeader name="T"/>
  <pous><pou name="P" pouType="program">
    <interface><localVars/></interface>
    <body><LD><network localId="1">
      <leftPowerRail localId="10" height="2">
        <position x="0" y="0"/>
        <connectionPointOut><relPosition x="2" y="1"/></connectionPointOut>
      </leftPowerRail>
      <contact localId="21" negated="{neg}" edge="none">
        <position x="50" y="0"/>
        <connectionPointIn>
          <relPosition x="0" y="1"/>
          <connection refLocalId="10"/>
        </connectionPointIn>
        <connectionPointOut><relPosition x="2" y="1"/></connectionPointOut>
        <variable>{var_a}</variable>
      </contact>
      {extra}
      <coil localId="30" negated="false" storage="none" edge="none">
        <position x="200" y="0"/>
        <connectionPointIn>
          <relPosition x="0" y="1"/>
          <connection refLocalId="{coil_ref}"/>
        </connectionPointIn>
        <variable>SORTIE</variable>
      </coil>
      <rightPowerRail localId="40" height="2">
        <position x="250" y="0"/>
        <connectionPointIn>
          <relPosition x="0" y="1"/>
          <connection refLocalId="30"/>
        </connectionPointIn>
      </rightPowerRail>
    </network></LD></body>
  </pou></pous>
</project>"#,
            neg = negated_a,
        );

        let proj = parse_project(&xml).unwrap();
        match &proj.pous[0].body {
            Body::Ld(ld) => ld.networks[0].clone(),
            _ => panic!(),
        }
    }

    #[test]
    fn diff_identiques_aucun_highlight() {
        let net = make_network("CAP", None, false);
        let svg = render_diff(&net, &net);
        // Aucun élément coloré en vert/rouge/jaune
        assert!(!svg.contains("#16a34a"), "pas de vert si identiques");
        assert!(!svg.contains("#dc2626"), "pas de rouge si identiques");
        assert!(!svg.contains("#d97706"), "pas de jaune si identiques");
    }

    #[test]
    fn diff_contact_modifie() {
        let net_a = make_network("CAP", None, false); // contact NO
        let net_b = make_network("CAP", None, true); // contact NF : même id, contenu différent
        let svg = render_diff(&net_a, &net_b);
        assert!(svg.contains("#d97706"), "contact modifié → jaune");
    }

    #[test]
    fn diff_contact_ajoute() {
        let net_a = make_network("CAP", None, false);
        let net_b = make_network("CAP", Some("SECURITE"), false);
        let svg = render_diff(&net_a, &net_b);
        assert!(svg.contains("#16a34a"), "nouveau contact → vert");
    }

    #[test]
    fn diff_contact_supprime() {
        let net_a = make_network("CAP", Some("SECURITE"), false);
        let net_b = make_network("CAP", None, false);
        let svg = render_diff(&net_a, &net_b);
        assert!(svg.contains("#dc2626"), "contact supprimé → rouge");
        assert!(
            svg.contains("SECURITE"),
            "variable supprimée visible dans le SVG"
        );
    }
}
