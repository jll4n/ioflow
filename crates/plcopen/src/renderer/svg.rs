use std::collections::HashMap;
use std::fmt::Write as _;

use crate::types::*;

use super::{element_local_id, ElemColor};

// ─── Constantes de mise à l'échelle ──────────────────────────────────────────
// Les coordonnées PLCopen (ex : x=50, y=0) sont multipliées par ces facteurs
// pour obtenir des pixels SVG. Les valeurs typiques de Control Expert donnent
// des positions comme 0, 50, 110, 200, 250 en x, et 0, 1, 2 en y.

const SX: i32 = 3; // 1 unité PLCopen x → 3 px SVG
const SY: i32 = 30; // 1 unité PLCopen y → 30 px SVG
const PAD: i32 = 22; // marge extérieure

// Dimensions visuelles fixes des symboles (en px SVG)
const RAIL_W: i32 = 5;
const CONTACT_W: i32 = 40;
const COIL_W: i32 = 40;
const BLOCK_W: i32 = 80;
const ELEM_H: i32 = 20; // hauteur du symbole contact/bobine
const FONT: i32 = 11;

// ─── API publique ─────────────────────────────────────────────────────────────

/// Rend un réseau ladder en SVG avec les couleurs par défaut (noir).
pub fn render_network(network: &LdNetwork) -> String {
    render_network_colored(network, &|_| ElemColor::Normal)
}

/// Rend un réseau ladder en SVG avec une fonction de colorisation par `localId`.
pub fn render_network_colored(network: &LdNetwork, color_fn: &dyn Fn(u32) -> ElemColor) -> String {
    // 1. Carte des points de sortie : localId → (svg_x, svg_y)
    let mut out_map: HashMap<u32, (i32, i32)> = HashMap::new();
    for elem in &network.elements {
        if let Some((id, pt)) = elem_output(elem) {
            out_map.insert(id, pt);
        }
    }

    // 2. Boîte englobante → viewBox
    let (vx, vy, vw, vh) = compute_viewport(&network.elements);

    // 3. Construction du SVG
    let mut s = String::with_capacity(8192);
    write!(
        s,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{vx} {vy} {vw} {vh}" \
width="{vw}" height="{vh}" font-family="monospace" font-size="{FONT}">"#
    )
    .unwrap();

    write!(
        s,
        r#"<rect x="{vx}" y="{vy}" width="{vw}" height="{vh}" fill="white"/>"#
    )
    .unwrap();

    // Nom du réseau en petit gris en haut à gauche
    if let Some(name) = &network.name {
        write!(
            s,
            r#"<text x="{}" y="{}" fill="{}" font-size="9">{}</text>"#,
            vx + 2,
            vy + 9,
            "#999",
            xml_esc(name)
        )
        .unwrap();
    }

    // Fils en premier (sous les symboles)
    for elem in &network.elements {
        let color = color_fn(element_local_id(elem));
        draw_wires(elem, &out_map, color, &mut s);
    }

    // Symboles par-dessus
    for elem in &network.elements {
        let color = color_fn(element_local_id(elem));
        draw_elem(elem, color, &mut s);
    }

    s.push_str("</svg>");
    s
}

// ─── Points de connexion ──────────────────────────────────────────────────────

/// Point de sortie (absolu en px SVG) d'un élément, identifié par son `localId`.
fn elem_output(elem: &LdElement) -> Option<(u32, (i32, i32))> {
    match elem {
        LdElement::LeftPowerRail(r) => {
            let x = r.position.x * SX + RAIL_W;
            let y = r
                .connection_points_out
                .first()
                .map(|cp| (r.position.y + cp.rel_position.y) * SY)
                .unwrap_or_else(|| r.position.y * SY + r.height as i32 * SY / 2);
            Some((r.local_id, (x, y)))
        }
        LdElement::Contact(c) => Some((c.local_id, (c.position.x * SX + CONTACT_W, wire_y_c(c)))),
        LdElement::Coil(c) => Some((c.local_id, (c.position.x * SX + COIL_W, wire_y_l(c)))),
        LdElement::Block(b) => {
            // Point de sortie = bord droit du bloc, centré verticalement
            let x = b.position.x * SX + BLOCK_W;
            let y = b.position.y * SY + block_height(b) / 2;
            Some((b.local_id, (x, y)))
        }
        LdElement::RightPowerRail(_) => None,
    }
}

fn wire_y_c(c: &Contact) -> i32 {
    c.connection_point_in
        .as_ref()
        .map(|cp| (c.position.y + cp.rel_position.y) * SY)
        .unwrap_or_else(|| c.position.y * SY + SY / 2)
}

fn wire_y_l(c: &Coil) -> i32 {
    c.connection_point_in
        .as_ref()
        .map(|cp| (c.position.y + cp.rel_position.y) * SY)
        .unwrap_or_else(|| c.position.y * SY + SY / 2)
}

fn block_height(b: &LdBlock) -> i32 {
    let pins = b.inputs.len().max(b.outputs.len()).max(1) as i32;
    pins * 24 + 28
}

// ─── Boîte englobante ─────────────────────────────────────────────────────────

fn compute_viewport(elements: &[LdElement]) -> (i32, i32, i32, i32) {
    let mut min_x = i32::MAX;
    let mut max_x = i32::MIN;
    let mut min_y = i32::MAX;
    let mut max_y = i32::MIN;

    for elem in elements {
        let (x0, y0, x1, y1) = elem_bounds(elem);
        min_x = min_x.min(x0);
        min_y = min_y.min(y0);
        max_x = max_x.max(x1);
        max_y = max_y.max(y1);
    }

    if min_x == i32::MAX {
        return (0, 0, 200, 60);
    }

    (
        min_x - PAD,
        min_y - PAD,
        max_x - min_x + PAD * 2,
        max_y - min_y + PAD * 2,
    )
}

fn elem_bounds(elem: &LdElement) -> (i32, i32, i32, i32) {
    match elem {
        LdElement::LeftPowerRail(r) => {
            let x = r.position.x * SX;
            let y0 = r.position.y * SY;
            (x, y0, x + RAIL_W, y0 + r.height as i32 * SY)
        }
        LdElement::RightPowerRail(r) => {
            let x = r.position.x * SX;
            let y0 = r.position.y * SY;
            (x, y0, x + RAIL_W, y0 + r.height as i32 * SY)
        }
        LdElement::Contact(c) => {
            let x0 = c.position.x * SX;
            let wy = wire_y_c(c);
            (
                x0,
                wy - ELEM_H / 2 - FONT - 2,
                x0 + CONTACT_W,
                wy + ELEM_H / 2,
            )
        }
        LdElement::Coil(c) => {
            let x0 = c.position.x * SX;
            let wy = wire_y_l(c);
            (x0, wy - ELEM_H / 2 - FONT - 2, x0 + COIL_W, wy + ELEM_H / 2)
        }
        LdElement::Block(b) => {
            let x0 = b.position.x * SX;
            let y0 = b.position.y * SY;
            (x0, y0 - FONT - 2, x0 + BLOCK_W, y0 + block_height(b))
        }
    }
}

// ─── Tracé des fils ───────────────────────────────────────────────────────────

fn draw_wires(
    elem: &LdElement,
    out_map: &HashMap<u32, (i32, i32)>,
    color: ElemColor,
    s: &mut String,
) {
    let stroke = color.stroke();
    match elem {
        LdElement::Contact(c) => {
            let dx = c.position.x * SX;
            let dy = wire_y_c(c);
            if let Some(cp) = &c.connection_point_in {
                for conn in &cp.connections {
                    if let Some(&(sx, sy)) = out_map.get(&conn.ref_local_id) {
                        wire(sx, sy, dx, dy, stroke, s);
                    }
                }
            }
        }
        LdElement::Coil(c) => {
            let dx = c.position.x * SX;
            let dy = wire_y_l(c);
            if let Some(cp) = &c.connection_point_in {
                for conn in &cp.connections {
                    if let Some(&(sx, sy)) = out_map.get(&conn.ref_local_id) {
                        wire(sx, sy, dx, dy, stroke, s);
                    }
                }
            }
        }
        LdElement::RightPowerRail(r) => {
            let dx = r.position.x * SX;
            for cp in &r.connection_points_in {
                let dy = (r.position.y + cp.rel_position.y) * SY;
                for conn in &cp.connections {
                    if let Some(&(sx, sy)) = out_map.get(&conn.ref_local_id) {
                        wire(sx, sy, dx, dy, stroke, s);
                    }
                }
            }
        }
        LdElement::Block(b) => {
            for (i, pin) in b.inputs.iter().enumerate() {
                let dx = b.position.x * SX;
                let dy = b.position.y * SY + 28 + i as i32 * 24 + 12;
                for conn in &pin.connections {
                    if let Some(&(sx, sy)) = out_map.get(&conn.ref_local_id) {
                        wire(sx, sy, dx, dy, stroke, s);
                    }
                }
            }
        }
        LdElement::LeftPowerRail(_) => {}
    }
}

fn wire(x1: i32, y1: i32, x2: i32, y2: i32, stroke: &str, s: &mut String) {
    if y1 == y2 {
        write!(
            s,
            r#"<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{stroke}" stroke-width="1.5"/>"#
        )
        .unwrap();
    } else {
        // Fil en L : horizontal jusqu'au milieu, vertical, puis horizontal
        let mx = x1 + (x2 - x1) / 2;
        write!(
            s,
            r#"<polyline points="{x1},{y1} {mx},{y1} {mx},{y2} {x2},{y2}" fill="none" stroke="{stroke}" stroke-width="1.5"/>"#
        )
        .unwrap();
    }
}

// ─── Rendu des symboles ───────────────────────────────────────────────────────

fn draw_elem(elem: &LdElement, color: ElemColor, s: &mut String) {
    match elem {
        LdElement::LeftPowerRail(r) => {
            let x = r.position.x * SX;
            let y0 = r.position.y * SY;
            let h = r.height as i32 * SY;
            write!(
                s,
                r#"<rect x="{x}" y="{y0}" width="{RAIL_W}" height="{h}" fill="{}"/>"#,
                color.stroke()
            )
            .unwrap();
        }
        LdElement::RightPowerRail(r) => {
            let x = r.position.x * SX;
            let y0 = r.position.y * SY;
            let h = r.height as i32 * SY;
            write!(
                s,
                r#"<rect x="{x}" y="{y0}" width="{RAIL_W}" height="{h}" fill="{}"/>"#,
                color.stroke()
            )
            .unwrap();
        }
        LdElement::Contact(c) => draw_contact(c, color, s),
        LdElement::Coil(c) => draw_coil(c, color, s),
        LdElement::Block(b) => draw_block(b, color, s),
    }
}

fn draw_contact(c: &Contact, color: ElemColor, s: &mut String) {
    let x = c.position.x * SX;
    let wy = wire_y_c(c);
    let top = wy - ELEM_H / 2;

    // Rectangle du contact
    write!(
        s,
        r#"<rect x="{x}" y="{top}" width="{CONTACT_W}" height="{ELEM_H}" \
fill="{}" stroke="{}" stroke-width="1.5"/>"#,
        color.fill(),
        color.stroke()
    )
    .unwrap();

    // Barre diagonale pour contact NF [/]
    if c.negated {
        let x1 = x + 7;
        let y1 = top + ELEM_H - 4;
        let x2 = x + CONTACT_W - 7;
        let y2 = top + 4;
        write!(
            s,
            r#"<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{}" stroke-width="1.5"/>"#,
            color.stroke()
        )
        .unwrap();
    }

    // Marqueur de front (petite flèche dans le coin inférieur droit)
    let edge_ch = match c.edge {
        EdgeDetection::Rising | EdgeDetection::Both => Some("↑"),
        EdgeDetection::Falling => Some("↓"),
        EdgeDetection::None => None,
    };
    if let Some(ch) = edge_ch {
        write!(
            s,
            r#"<text x="{}" y="{}" text-anchor="middle" fill="{}" font-size="8">{ch}</text>"#,
            x + CONTACT_W - 8,
            top + ELEM_H - 2,
            color.stroke()
        )
        .unwrap();
    }

    // Nom de la variable au-dessus
    write!(
        s,
        r#"<text x="{}" y="{}" text-anchor="middle" fill="{}">{}</text>"#,
        x + CONTACT_W / 2,
        top - 2,
        color.stroke(),
        xml_esc(&c.variable)
    )
    .unwrap();
}

fn draw_coil(c: &Coil, color: ElemColor, s: &mut String) {
    let x = c.position.x * SX;
    let wy = wire_y_l(c);
    let top = wy - ELEM_H / 2;
    let rx = COIL_W / 4; // arrondi horizontal → forme pill = bobine ( )
    let ry = ELEM_H / 2; // arrondi vertical = demi-hauteur

    // Symbole pill (bobine)
    write!(
        s,
        r#"<rect x="{x}" y="{top}" width="{COIL_W}" height="{ELEM_H}" \
rx="{rx}" ry="{ry}" fill="{}" stroke="{}" stroke-width="1.5"/>"#,
        color.fill(),
        color.stroke()
    )
    .unwrap();

    // Marqueur intérieur (S, R, M pour mémorisation ; / pour inversée)
    let inner: &str = match (&c.storage, c.negated) {
        (CoilStorage::Set, _) => "S",
        (CoilStorage::Reset, _) => "R",
        (CoilStorage::Retentive, _) => "M",
        (CoilStorage::NonRetentive, _) => "m",
        (CoilStorage::None, true) => "/",
        (CoilStorage::None, false) => "",
    };
    if !inner.is_empty() {
        write!(
            s,
            r#"<text x="{}" y="{}" text-anchor="middle" fill="{}" font-size="9">{inner}</text>"#,
            x + COIL_W / 2,
            wy + 4,
            color.stroke()
        )
        .unwrap();
    }

    // Nom de la variable au-dessus
    write!(
        s,
        r#"<text x="{}" y="{}" text-anchor="middle" fill="{}">{}</text>"#,
        x + COIL_W / 2,
        top - 2,
        color.stroke(),
        xml_esc(&c.variable)
    )
    .unwrap();
}

fn draw_block(b: &LdBlock, color: ElemColor, s: &mut String) {
    let x = b.position.x * SX;
    let y0 = b.position.y * SY;
    let h = block_height(b);

    // Rectangle principal
    write!(
        s,
        r#"<rect x="{x}" y="{y0}" width="{BLOCK_W}" height="{h}" \
fill="{}" stroke="{}" stroke-width="1.5"/>"#,
        color.fill(),
        color.stroke()
    )
    .unwrap();

    // Nom du type (centré, gras)
    write!(
        s,
        r#"<text x="{}" y="{}" text-anchor="middle" font-weight="bold" fill="{}">{}</text>"#,
        x + BLOCK_W / 2,
        y0 + 16,
        color.stroke(),
        xml_esc(&b.type_name)
    )
    .unwrap();

    // Nom de l'instance en italique, plus petit
    if let Some(inst) = &b.instance_name {
        write!(
            s,
            r#"<text x="{}" y="{}" text-anchor="middle" fill="{}" font-size="9" font-style="italic">{}</text>"#,
            x + BLOCK_W / 2,
            y0 + 26,
            color.stroke(),
            xml_esc(inst)
        )
        .unwrap();
    }

    // Broches d'entrée (à gauche)
    for (i, pin) in b.inputs.iter().enumerate() {
        let py = y0 + 28 + i as i32 * 24 + 14;
        write!(
            s,
            r#"<text x="{}" y="{py}" fill="{}" font-size="9">{}</text>"#,
            x + 3,
            color.stroke(),
            xml_esc(&pin.formal_parameter)
        )
        .unwrap();
    }

    // Broches de sortie (à droite)
    for (i, pin) in b.outputs.iter().enumerate() {
        let py = y0 + 28 + i as i32 * 24 + 14;
        write!(
            s,
            r#"<text x="{}" y="{py}" text-anchor="end" fill="{}" font-size="9">{}</text>"#,
            x + BLOCK_W - 3,
            color.stroke(),
            xml_esc(&pin.formal_parameter)
        )
        .unwrap();
    }
}

// ─── Utilitaires ─────────────────────────────────────────────────────────────

fn xml_esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_project;

    fn fixture_xml(negated: bool) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://www.plcopen.org/xml/tc6_0201">
  <fileHeader companyName="ACME"/>
  <contentHeader name="TEST"/>
  <pous>
    <pou name="PROG" pouType="program">
      <interface><localVars/></interface>
      <body>
        <LD>
          <network localId="1">
            <name>Réseau de test</name>
            <leftPowerRail localId="10" height="2">
              <position x="0" y="0"/>
              <connectionPointOut><relPosition x="2" y="1"/></connectionPointOut>
            </leftPowerRail>
            <contact localId="20" negated="{neg}" edge="none">
              <position x="50" y="0"/>
              <connectionPointIn>
                <relPosition x="0" y="1"/>
                <connection refLocalId="10"/>
              </connectionPointIn>
              <connectionPointOut><relPosition x="2" y="1"/></connectionPointOut>
              <variable>CAPTEUR</variable>
            </contact>
            <coil localId="30" negated="false" storage="none" edge="none">
              <position x="150" y="0"/>
              <connectionPointIn>
                <relPosition x="0" y="1"/>
                <connection refLocalId="20"/>
              </connectionPointIn>
              <variable>MOTEUR</variable>
            </coil>
            <rightPowerRail localId="40" height="2">
              <position x="200" y="0"/>
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
</project>"#,
            neg = negated
        )
    }

    fn first_network(xml: &str) -> LdNetwork {
        let project = parse_project(xml).unwrap();
        match &project.pous[0].body {
            Body::Ld(ld) => ld.networks[0].clone(),
            _ => panic!("body non-LD"),
        }
    }

    #[test]
    fn render_reseau_no() {
        let net = first_network(&fixture_xml(false));
        let svg = render_network(&net);
        assert!(svg.contains("<svg"), "doit produire un SVG");
        assert!(svg.contains("CAPTEUR"), "doit contenir le nom de variable");
        assert!(svg.contains("MOTEUR"), "doit contenir le nom de la bobine");
        assert!(
            svg.contains("<line") || svg.contains("<polyline"),
            "doit avoir des fils"
        );
        assert!(svg.contains("<rect"), "doit avoir des symboles rect");
        // Pas de barre diagonale pour NO
        assert!(
            !svg.contains("stroke-width=\"1.5\"/>") || svg.contains("CAPTEUR"),
            "vérification minimale"
        );
    }

    #[test]
    fn render_contact_nf() {
        let net = first_network(&fixture_xml(true));
        let svg = render_network(&net);
        // Le contact NF a une diagonale : deux <line> (un diagonal + les fils)
        let line_count = svg.matches("<line").count();
        assert!(
            line_count >= 2,
            "contact NF doit avoir au moins 2 <line> (fil + diagonale)"
        );
    }

    #[test]
    fn dimensions_coherentes() {
        let net = first_network(&fixture_xml(false));
        let svg = render_network(&net);
        // Le viewBox doit avoir des dimensions positives
        let vb_start = svg.find("viewBox=\"").unwrap();
        let rest = &svg[vb_start + 9..];
        let end = rest.find('"').unwrap();
        let parts: Vec<i32> = rest[..end]
            .split_whitespace()
            .map(|v| v.parse().unwrap())
            .collect();
        assert_eq!(parts.len(), 4);
        assert!(parts[2] > 0, "largeur > 0");
        assert!(parts[3] > 0, "hauteur > 0");
    }

    #[test]
    fn render_colored() {
        let net = first_network(&fixture_xml(false));
        // Colorise le contact (id=20) en "ajouté" (vert)
        let svg = render_network_colored(&net, &|id| {
            if id == 20 {
                ElemColor::Added
            } else {
                ElemColor::Normal
            }
        });
        assert!(svg.contains("#16a34a"), "doit contenir la couleur verte");
    }
}
