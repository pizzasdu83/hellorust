#![allow(warnings)]
mod bindings;

use bindings::Guest;
use klave;
use serde_json::{Value, json};

struct Component;

impl Guest for Component {

	fn register_routes() {
		klave::router::add_user_query("load-from-ledger");
		klave::router::add_user_query("fetch-all-scores"); // ✅ nouvelle route
		klave::router::add_user_transaction("insert-in-ledger");
	}

	fn load_from_ledger(cmd: String) {
		let Ok(v) = serde_json::from_str::<Value>(&cmd) else {
			klave::notifier::send_string(&format!("failed to parse '{}' as json", cmd));
			return;
		};
		let key = v["key"].as_str().unwrap();
		let Ok(res) = klave::ledger::get_table("my_table").get(&key) else {
			klave::notifier::send_string(&format!("failed to read from ledger: '{}'", cmd));
			return;
		};
		let msg = if res.is_empty() {
			format!("the key '{}' was not found in table my_table", cmd)
		} else {
			let result_as_json = json!({
				"value": String::from_utf8(res).unwrap_or("!! utf8 parsing error !!".to_owned()),
			});
			format!("{}", result_as_json.to_string())
		};
		klave::notifier::send_string(&msg);
	}

	fn insert_in_ledger(cmd: String) {
		let Ok(v) = serde_json::from_str::<Value>(&cmd) else {
			klave::notifier::send_string(&format!("failed to parse '{}' as json", cmd));
			klave::router::cancel_transaction();
			return;
		};
		let key = v["key"].as_str().unwrap();
		let value = v["value"].as_str().unwrap().as_bytes();
		match klave::ledger::get_table("my_table").set(&key, &value) {
			Err(e) => {
				klave::notifier::send_string(&format!("failed to write to ledger: '{}'", e));
				klave::router::cancel_transaction();
				return;
			}
			_ => {}
		}

		let result_as_json = json!({
			"inserted": true,
			"key": key,
			"value": value
		});
		klave::notifier::send_string(&result_as_json.to_string());
	}

	// ✅ Nouvelle fonction : fetch-all-scores
	fn fetch_all_scores(_: String) {
		let mut scores = Vec::new();

		let Ok(mut cursor) = klave::ledger::get_table("my_table").cursor() else {
			klave::notifier::send_string("failed to get cursor from ledger");
			return;
		};

		while let Some((key, val)) = cursor.next() {
			let key_str = match std::str::from_utf8(&key) {
				Ok(s) => s,
				Err(_) => continue,
			};

			if key_str.starts_with("score:") {
				let Ok(v) = serde_json::from_slice::<serde_json::Value>(&val) else {
					continue;
				};

				let name = v["name"].as_str().unwrap_or("???").to_string();
				let score = v["score"].as_u64().unwrap_or(0);

				scores.push(json!({
					"name": name,
					"score": score
				}));
			}
		}

		scores.sort_by(|a, b| {
			let sa = a["score"].as_u64().unwrap_or(0);
			let sb = b["score"].as_u64().unwrap_or(0);
			sb.cmp(&sa)
		});

		let result = json!({ "success": true, "value": scores });
		klave::notifier::send_string(&result.to_string());
	}
}

bindings::export!(Component with_types_in bindings);
