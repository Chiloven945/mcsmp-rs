#[allow(dead_code)]
/// Asserts that request values contain exactly `methods` in order.
pub fn assert_methods(requests: &[serde_json::Value], methods: &[&str]) {
    let actual: Vec<_> = requests
        .iter()
        .map(|request| {
            request["method"]
                .as_str()
                .expect("request method must be a string")
        })
        .collect();
    assert_eq!(actual, methods);
}
