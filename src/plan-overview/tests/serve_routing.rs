// MODE: DEV
// The whole point of this crate's --serve mode is that every link on a
// rendered page is a real, working navigation. Before this test existed, it
// was not: the server always returned the one page rendered at startup
// regardless of the request path, and every internal link pointed at a
// #hash fragment a browser never even sends to the server. This test proves
// the fix end to end -- a real TCP client hitting a real running server --
// rather than only unit-testing the router/page functions in isolation.
use plan_overview::serve::{serve_on_host_port, state_stream};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

const STATE_JSON: &str = r#"{"identity":{"title":"Demo plan","uiAffected":"yes","reviewStatus":"✅ approved","description":"desc"},"goals":[{"id":"g1","outcome":"ship it","testingRequirement":"yes"}],"steps":[{"goal":"g1","step":"01-step-a","unit":"W01","type":"source","target":"src/lib.rs","companion":null,"status":"completed","instructions":"do it","criteria":"works"}],"edges":[],"testingMarks":[],"coverage":[],"findings":[{"id":"AR-01","item":"a gap","change":"fix it","status":"open","workUnit":"W01","cycle":"current"}],"cycles":1,"reviewTarget":2,"generatedAt":"now","generatedBy":"test"}"#;

fn get(port: u16, path: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    write!(stream, "GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).ok();
    response
}

#[test]
fn each_path_is_routed_and_rendered_independently() {
    let stream = state_stream(STATE_JSON.to_string());
    let server = serve_on_host_port(stream, "127.0.0.1", 0).expect("server starts");
    let port: u16 = server
        .address()
        .rsplit(':')
        .next()
        .unwrap()
        .parse()
        .unwrap();

    let overview = get(port, "/");
    assert!(overview.contains("Demo plan"), "{overview}");
    assert!(
        overview.contains(">g1<") || overview.contains("g1</a>"),
        "{overview}"
    );

    let goal = get(port, "/goal/g1");
    assert!(goal.contains("ship it"), "{goal}");
    assert!(goal.contains("01-step-a"), "{goal}");
    assert!(
        !goal.contains("id=\"overview\""),
        "goal page rendered the overview article instead of the goal page: {goal}"
    );

    let unit = get(port, "/unit/W01");
    assert!(unit.contains("src/lib.rs"), "{unit}");
    assert!(
        unit.contains("g1"),
        "unit page should link back to its goal: {unit}"
    );

    let finding = get(port, "/finding/AR-01");
    assert!(finding.contains("a gap"), "{finding}");
    assert!(finding.contains("fix it"), "{finding}");

    // A path naming nothing real falls back to the overview page rather than
    // hanging, erroring, or echoing an unrelated previous response.
    let unknown = get(port, "/goal/does-not-exist");
    assert!(unknown.contains("Demo plan"), "{unknown}");
}
