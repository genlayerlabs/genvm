fn main() {
    afl::fuzz!(|data: &[u8]| {
        let Ok(data_as_str) = std::str::from_utf8(data) else {
            return;
        };

        if serde_json::from_str::<serde_json::Value>(data_as_str).is_err() {
            return;
        };

        for (i, _) in data_as_str.char_indices() {
            let partial = &data_as_str[0..i];
            let completed = genvm_modules::complete_json(partial);

            if let Err(err) = serde_json::from_str::<serde_json::Value>(&completed) {
                panic!(
                    "Completed JSON is invalid.\nPartial: {:?}\nCompleted: {:?}\nError: {:?}\nCompleted raw: {}",
                    partial,
                    completed,
                    err,
                    data_as_str
                );
            };
        }
    })
}
