//! Generate the versioned sample used by manual desktop/web acceptance.
use erlu_core::*;
use serde_json::{Value, json};
fn call(e: &mut Engine, name: &str, args: Value) -> Response {
    e.call(name, args, None).unwrap()
}
fn ep(id: Id, port: Port) -> Endpoint {
    Endpoint::new(Anchor::Block { id, port, t: 0.5 })
}
fn edge(port: Port) -> Endpoint {
    Endpoint::new(Anchor::Boundary { port, t: 0.5 })
}
fn main() {
    let mut e = Engine::new(Document::with_meta(Metadata {
        title: "Fulfil customer orders".into(),
        organization: "Example.Org".into(),
        author: "Alex Morgan".into(),
        initials: "AM".into(),
        currency: "€".into(),
        currency_placement: "1 ?".into(),
        ..Metadata::default()
    }))
    .unwrap();
    call(
        &mut e,
        "block_update",
        json!({"id":2,"title":"Fulfil customer orders"}),
    );
    for (port, label) in [
        (Port::Input, "Customer orders"),
        (Port::Control, "Fulfilment policy"),
        (Port::Mechanism, "Operations team"),
    ] {
        call(
            &mut e,
            "arrow_create",
            json!({"from":edge(port),"to":ep(2,port),"label":label}),
        );
    }
    call(
        &mut e,
        "arrow_create",
        json!({"from":ep(2,Port::Output),"to":edge(Port::Output),"label":"Completed orders"}),
    );
    call(&mut e, "decompose", json!({"block":2}));
    let d = e.view.diagram;
    let mut ids = vec![];
    for (i, title) in ["Receive orders", "Prepare shipment", "Deliver goods"]
        .iter()
        .enumerate()
    {
        let id = call(
            &mut e,
            "block_create",
            json!({"pos":{"x":-105.+i as f64*80.,"y":-45.+i as f64*38.},"title":title}),
        )
        .result["id"]
            .as_u64()
            .unwrap();
        ids.push(id);
        call(
            &mut e,
            "block_update",
            json!({"id":id,"price":10.+i as f64*5.}),
        );
    }
    let stubs = e.document.diagrams[&d]
        .arrows
        .values()
        .map(|a| (a.id, a.interface.as_ref().unwrap().port))
        .collect::<Vec<_>>();
    for (id, port) in stubs {
        let target = if port == Port::Output {
            *ids.last().unwrap()
        } else {
            ids[0]
        };
        call(
            &mut e,
            "interface_connect",
            json!({"id":id,"endpoint":ep(target,port)}),
        );
    }
    for (pair, label) in ids.windows(2).zip(["Accepted orders", "Packed shipments"]) {
        call(
            &mut e,
            "arrow_create",
            json!({"from":ep(pair[0],Port::Output),"to":ep(pair[1],Port::Input),"label":label}),
        );
    }
    call(&mut e, "calculate_prices", json!({}));
    call(&mut e, "navigate_up", json!({}));
    println!("{}", e.serialize_document());
}
