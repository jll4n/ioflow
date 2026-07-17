/// Génère deux fichiers SVG dans target/ pour prévisualiser le renderer ladder :
///   target/ladder_single.svg  — réseau simple (contact NO, contact NF, bobine)
///   target/ladder_diff.svg    — diff entre la version A et B (un contact ajouté)
///
/// Usage : cargo run -p plcopen --example preview
use plcopen::{
    parser::parse_project,
    renderer::{diff::render_diff, svg::render_network},
    Body,
};

const XML_A: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://www.plcopen.org/xml/tc6_0201">
  <fileHeader companyName="ACME" productName="Control Expert" productVersion="15.0"/>
  <contentHeader name="CONVOYEUR" version="1.0"/>
  <pous>
    <pou name="PROG_CONVOYEUR" pouType="program">
      <interface>
        <localVars>
          <variable name="CAPTEUR_ARRIVEE"><type><BOOL/></type></variable>
          <variable name="DEFAUT_MOTEUR"><type><BOOL/></type></variable>
          <variable name="MOTEUR_ON"><type><BOOL/></type></variable>
        </localVars>
      </interface>
      <body>
        <LD>
          <network localId="1">
            <name>Démarrage convoyeur</name>
            <leftPowerRail localId="10" height="2">
              <position x="0" y="0"/>
              <connectionPointOut><relPosition x="2" y="1"/></connectionPointOut>
            </leftPowerRail>
            <contact localId="20" negated="false" edge="none">
              <position x="50" y="0"/>
              <connectionPointIn>
                <relPosition x="0" y="1"/>
                <connection refLocalId="10"/>
              </connectionPointIn>
              <connectionPointOut><relPosition x="2" y="1"/></connectionPointOut>
              <variable>CAPTEUR_ARRIVEE</variable>
            </contact>
            <contact localId="21" negated="true" edge="none">
              <position x="110" y="0"/>
              <connectionPointIn>
                <relPosition x="0" y="1"/>
                <connection refLocalId="20"/>
              </connectionPointIn>
              <connectionPointOut><relPosition x="2" y="1"/></connectionPointOut>
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

// Version B : on ajoute un contact de sécurité URGENCE (NF) entre DEFAUT_MOTEUR et la bobine
const XML_B: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://www.plcopen.org/xml/tc6_0201">
  <fileHeader companyName="ACME" productName="Control Expert" productVersion="15.0"/>
  <contentHeader name="CONVOYEUR" version="1.0"/>
  <pous>
    <pou name="PROG_CONVOYEUR" pouType="program">
      <interface>
        <localVars>
          <variable name="CAPTEUR_ARRIVEE"><type><BOOL/></type></variable>
          <variable name="DEFAUT_MOTEUR"><type><BOOL/></type></variable>
          <variable name="URGENCE"><type><BOOL/></type></variable>
          <variable name="MOTEUR_ON"><type><BOOL/></type></variable>
        </localVars>
      </interface>
      <body>
        <LD>
          <network localId="1">
            <name>Démarrage convoyeur</name>
            <leftPowerRail localId="10" height="2">
              <position x="0" y="0"/>
              <connectionPointOut><relPosition x="2" y="1"/></connectionPointOut>
            </leftPowerRail>
            <contact localId="20" negated="false" edge="none">
              <position x="50" y="0"/>
              <connectionPointIn>
                <relPosition x="0" y="1"/>
                <connection refLocalId="10"/>
              </connectionPointIn>
              <connectionPointOut><relPosition x="2" y="1"/></connectionPointOut>
              <variable>CAPTEUR_ARRIVEE</variable>
            </contact>
            <contact localId="21" negated="true" edge="none">
              <position x="110" y="0"/>
              <connectionPointIn>
                <relPosition x="0" y="1"/>
                <connection refLocalId="20"/>
              </connectionPointIn>
              <connectionPointOut><relPosition x="2" y="1"/></connectionPointOut>
              <variable>DEFAUT_MOTEUR</variable>
            </contact>
            <contact localId="22" negated="true" edge="none">
              <position x="160" y="0"/>
              <connectionPointIn>
                <relPosition x="0" y="1"/>
                <connection refLocalId="21"/>
              </connectionPointIn>
              <connectionPointOut><relPosition x="2" y="1"/></connectionPointOut>
              <variable>URGENCE</variable>
            </contact>
            <coil localId="30" negated="false" storage="none" edge="none">
              <position x="220" y="0"/>
              <connectionPointIn>
                <relPosition x="0" y="1"/>
                <connection refLocalId="22"/>
              </connectionPointIn>
              <variable>MOTEUR_ON</variable>
            </coil>
            <rightPowerRail localId="40" height="2">
              <position x="270" y="0"/>
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

fn ld_network(xml: &str) -> plcopen::LdNetwork {
    let project = parse_project(xml).expect("XML invalide");
    match &project.pous[0].body {
        Body::Ld(ld) => ld.networks[0].clone(),
        _ => panic!("POU non-LD"),
    }
}

fn main() {
    // ── SVG réseau simple ────────────────────────────────────────────────────
    let net_a = ld_network(XML_A);
    let svg_single = render_network(&net_a);

    let out_single = "target/ladder_single.svg";
    std::fs::write(out_single, &svg_single).expect("écriture échouée");
    println!("✓  {out_single}  ({} octets)", svg_single.len());

    // ── SVG diff ─────────────────────────────────────────────────────────────
    let net_b = ld_network(XML_B);
    let svg_diff = render_diff(&net_a, &net_b);

    let out_diff = "target/ladder_diff.svg";
    std::fs::write(out_diff, &svg_diff).expect("écriture échouée");
    println!("✓  {out_diff}  ({} octets)", svg_diff.len());

    println!("\nOuvrez ces fichiers dans un navigateur pour visualiser le rendu.");
}
