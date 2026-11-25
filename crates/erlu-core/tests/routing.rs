use erlu_core::*;
use erlu_core::{render::route, routing::*};
use serde_json::json;

fn broken() -> Document {
    read_document(include_str!("fixtures/arrow-backtrack.idef0.json")).unwrap()
}
fn length(path: &[Point]) -> f64 {
    path.windows(2).map(|w| w[0].distance(w[1])).sum()
}
fn orthogonal(path: &[Point]) {
    assert!(
        path.windows(2)
            .all(|w| (w[0].x - w[1].x).abs() < 1e-7 || (w[0].y - w[1].y).abs() < 1e-7),
        "{path:?}"
    );
}

#[test]
fn attached_file_renders_without_the_down_and_back_up_excursion() {
    let doc = broken();
    let before = doc.clone();
    let a = &doc.diagrams[&1].arrows[&3];
    let path = route(&doc, 1, a);
    println!("Repaired path: {path:?}");
    let start = doc.endpoint(1, &a.from).unwrap();
    let end = doc.endpoint(1, &a.to).unwrap();
    assert_eq!(path.first(), Some(&start));
    assert_eq!(path.last(), Some(&end));
    orthogonal(&path);
    assert!(
        path.iter().all(|p| p.y <= start.y.max(end.y) + 1e-7),
        "{path:?}"
    );
    assert!(length(&path) < 30.0, "{path:?}");
    assert_eq!(
        doc, before,
        "Rendering must not rewrite the user's document"
    );
}

#[test]
fn cleanup_is_saved_with_the_drag_and_undo_restores_the_original() {
    let original = broken();
    let mut e = Engine::new(original.clone()).unwrap();
    e.call(
        "objects_move",
        json!({"positions":{"2":{"x":-35.0,"y":-21.0}}}),
        None,
    )
    .unwrap();
    let a = &e.document.diagrams[&1].arrows[&3];
    assert!(a.bends.iter().all(|p| p.y < 0.0), "{:?}", a.bends);
    assert_eq!(
        e.document.diagrams[&1].junctions,
        original.diagrams[&1].junctions
    );
    assert_eq!(e.document.diagrams[&1].arrows[&5].label_source, Some(3));
    orthogonal(&route(&e.document, 1, a));
    e.call("undo", json!({}), None).unwrap();
    assert_eq!(e.document, original);
    e.call("redo", json!({}), None).unwrap();
    let reopened = read_document(&e.serialize_document()).unwrap();
    assert_eq!(
        route(&reopened, 1, &reopened.diagrams[&1].arrows[&3]),
        route(&e.document, 1, &e.document.diagrams[&1].arrows[&3])
    );
}

#[test]
fn redundant_uturns_and_small_steps_are_removed_but_obstacle_detours_remain() {
    let u = vec![
        Point::new(0., 0.),
        Point::new(0., 20.),
        Point::new(10., 20.),
        Point::new(10., 0.),
    ];
    assert_eq!(
        simplify(u.clone(), &[]),
        vec![Point::new(0., 0.), Point::new(10., 0.)]
    );
    let obstacle = [(Point::new(3., -2.), Point::new(7., 15.))];
    assert_eq!(simplify(u.clone(), &obstacle), u);
    let step = vec![
        Point::new(0., 0.),
        Point::new(10., 0.),
        Point::new(10., 0.5),
        Point::new(20., 0.5),
        Point::new(20., 15.),
    ];
    assert_eq!(
        simplify(step, &[]),
        vec![
            Point::new(0., 0.),
            Point::new(20., 0.),
            Point::new(20., 15.)
        ]
    );
    let intentional = vec![
        Point::new(0., 0.),
        Point::new(10., 0.),
        Point::new(10., 8.),
        Point::new(20., 8.),
    ];
    assert_eq!(simplify(intentional.clone(), &[]), intentional);
}

#[test]
fn nearby_border_connections_snap_exactly_in_both_directions_on_every_side() {
    for port in [Port::Input, Port::Output, Port::Control, Port::Mechanism] {
        for reverse in [matches!(port, Port::Output)] {
            let mut e = Engine::default();
            e.call(
                "objects_move",
                json!({"positions":{"2":{"x":-40.0,"y":-20.0}}}),
                None,
            )
            .unwrap();
            let block = Endpoint::new(Anchor::Block {
                id: 2,
                port,
                t: 0.5,
            });
            let p = e.document.endpoint(1, &block).unwrap();
            let (lo, hi) = e.document.content();
            let t = match port {
                Port::Input | Port::Output => (p.y + 1.0 - lo.y) / (hi.y - lo.y),
                _ => (p.x + 1.0 - lo.x) / (hi.x - lo.x),
            };
            let border = Endpoint::new(Anchor::Boundary { port, t });
            let (from, to) = if reverse {
                (block, border)
            } else {
                (border, block)
            };
            let id = e
                .call("arrow_create", json!({"from":from,"to":to}), None)
                .unwrap()
                .result["id"]
                .as_u64()
                .unwrap();
            let arrow = &e.document.diagrams[&1].arrows[&id];
            let p = e.document.endpoint(1, &arrow.from).unwrap();
            let q = e.document.endpoint(1, &arrow.to).unwrap();
            match port {
                Port::Input | Port::Output => assert!((p.y - q.y).abs() < 1e-7),
                _ => assert!((p.x - q.x).abs() < 1e-7),
            }
            // Correct direction pairs have a single, genuinely straight segment.
            if matches!(
                (port, reverse),
                (Port::Output, true)
                    | (Port::Input, false)
                    | (Port::Control, false)
                    | (Port::Mechanism, false)
            ) {
                assert_eq!(route(&e.document, 1, arrow).len(), 2);
            }
        }
    }
}

#[test]
fn dragging_a_straight_end_segment_slides_ports_instead_of_creating_a_dogleg() {
    let mut e = Engine::default();
    let source = Endpoint::new(Anchor::Block {
        id: 2,
        port: Port::Output,
        t: 0.5,
    });
    let y = e.document.endpoint(1, &source).unwrap().y;
    let (lo, hi) = e.document.content();
    let to = Endpoint::new(Anchor::Boundary {
        port: Port::Output,
        t: (y - lo.y) / (hi.y - lo.y),
    });
    let id = e
        .call("arrow_create", json!({"from":source,"to":to}), None)
        .unwrap()
        .result["id"]
        .as_u64()
        .unwrap();
    let a = &e.document.diagrams[&1].arrows[&id];
    let path = route(&e.document, 1, a);
    assert_eq!(path.len(), 2);
    let edited = drag_segment(
        &e.document,
        1,
        a,
        path,
        0,
        Point::new(0., 3.),
        Grid::default(),
    );
    let path = route(&e.document, 1, &edited);
    assert_eq!(path.len(), 2);
    assert!((path[0].y - y - 3.).abs() < 1e-7);
}

#[test]
fn grid_is_opt_in_snaps_negative_coordinates_and_is_not_a_document_edit() {
    let mut e = Engine::default();
    let original = e.document.clone();
    assert!(!e.view.grid.enabled);
    e.call("grid_set", json!({"enabled":true,"spacing":5.0}), None)
        .unwrap();
    assert_eq!(
        e.view.grid.snap(Point::new(-7.7, 8.2)),
        Point::new(-10., 10.)
    );
    assert_eq!(e.document, original);
    assert_eq!(e.revision, 0);
    assert!(!e.dirty());
    let before = e.view.grid;
    assert!(
        e.call("grid_set", json!({"enabled":false,"spacing":0.0}), None)
            .is_err()
    );
    assert_eq!(e.view.grid, before);
    e.call("document_new", json!({"discard":true}), None)
        .unwrap();
    assert_eq!(e.view.grid, before);
    let mut endpoint = Endpoint::new(Anchor::Block {
        id: 2,
        port: Port::Output,
        t: 0.42,
    });
    snap_endpoint(&e.document, 1, &mut endpoint, e.view.grid);
    let p = e.document.endpoint(1, &endpoint).unwrap();
    assert!((p.y / 5.0 - (p.y / 5.0).round()).abs() < 1e-7);
}

#[test]
fn group_translation_keeps_manual_bends_with_their_junctions() {
    let mut doc = Document::default();
    for (id, pos) in [(3, Point::new(-100., -60.)), (4, Point::new(-40., -40.))] {
        doc.diagrams.get_mut(&1).unwrap().junctions.insert(
            id,
            Junction {
                id,
                pos,
                extra: Extra::new(),
            },
        );
    }
    doc.id_sequence = 5;
    let mut e = Engine::new(doc).unwrap();
    let id=e.call("arrow_create",json!({"from":{"anchor":{"kind":"junction","id":3}},"to":{"anchor":{"kind":"junction","id":4}}}),None).unwrap().result["id"].as_u64().unwrap();
    e.call(
        "arrow_update",
        json!({"id":id,"bends":[{"x":-90.0,"y":-60.0},{"x":-90.0,"y":-40.0}]}),
        None,
    )
    .unwrap();
    let path = route(&e.document, 1, &e.document.diagrams[&1].arrows[&id]);
    e.call(
        "objects_move",
        json!({"positions":{"3":{"x":-90.0,"y":-60.0},"4":{"x":-30.0,"y":-40.0}}}),
        None,
    )
    .unwrap();
    let moved = route(&e.document, 1, &e.document.diagrams[&1].arrows[&id]);
    assert_eq!(
        moved,
        path.into_iter()
            .map(|p| p + Point::new(10., 0.))
            .collect::<Vec<_>>()
    );
}

#[test]
fn inherited_boundary_coordinates_do_not_slide_with_an_end_segment() {
    let e = Engine::default();
    let mut arrow = Arrow {
        id: 9,
        from: Endpoint::new(Anchor::Boundary {
            port: Port::Input,
            t: 0.5,
        }),
        to: Endpoint::new(Anchor::Block {
            id: 2,
            port: Port::Input,
            t: 0.5,
        }),
        label: String::new(),
        label_source: None,
        label_offset: Point::default(),
        label_hidden: false,
        bends: vec![],
        interface: Some(Interface {
            parent_arrow: 8,
            port: Port::Input,
            t: 0.5,
            stub: false,
        }),
        call_ref: None,
        color: None,
        extra: Extra::new(),
    };
    let (lo, hi) = e.document.content();
    let p = e.document.endpoint(1, &arrow.to).unwrap();
    arrow.from.anchor = Anchor::Boundary {
        port: Port::Input,
        t: (p.y - lo.y) / (hi.y - lo.y),
    };
    let path = route(&e.document, 1, &arrow);
    assert_eq!(path.len(), 2);
    let edited = drag_segment(
        &e.document,
        1,
        &arrow,
        path,
        0,
        Point::new(0., 3.),
        Grid::default(),
    );
    assert_eq!(edited.from, arrow.from);
    assert_eq!(edited.interface, arrow.interface);
    assert_ne!(edited.to, arrow.to);
}

#[test]
fn near_aligned_block_ports_stay_straight_after_a_small_block_move() {
    let mut e = Engine::default();
    e.call(
        "objects_move",
        json!({"positions":{"2":{"x":-90.0,"y":-10.0}}}),
        None,
    )
    .unwrap();
    let other = e
        .call("block_create", json!({"pos":{"x":30.0,"y":-9.3}}), None)
        .unwrap()
        .result["id"]
        .as_u64()
        .unwrap();
    let id=e.call("arrow_create",json!({"from":{"anchor":{"kind":"block","id":2,"port":"output","t":0.5}},"to":{"anchor":{"kind":"block","id":other,"port":"input","t":0.5}}}),None).unwrap().result["id"].as_u64().unwrap();
    assert_eq!(
        route(&e.document, 1, &e.document.diagrams[&1].arrows[&id]).len(),
        2
    );
    let before = e.document.clone();
    e.call(
        "objects_move",
        json!({"positions":{"2":{"x":-90.0,"y":-9.5}}}),
        None,
    )
    .unwrap();
    let a = &e.document.diagrams[&1].arrows[&id];
    assert_eq!(route(&e.document, 1, a).len(), 2);
    assert_eq!(
        e.document.diagrams[&1].blocks[&other],
        before.diagrams[&1].blocks[&other]
    );
    e.call("undo", json!({}), None).unwrap();
    assert_eq!(e.document, before);
}
