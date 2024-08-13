use core::str;
use spin_sdk::{
    http::{IntoResponse, Request, Response},
    http_component,
    key_value::Store,
    llm::{infer, InferencingModel},
};
use serde::Deserialize;
use serde_json::{self, json};
use random_string::generate;

#[derive(Deserialize, Debug)]
pub struct SampleResponse {
    pub destination: String, 
    pub duration: String,
    pub num_people: String,
    pub activities: Vec<String>,
}

fn extract_tag(path: &str) -> Option<&str> {
    const PREFIX: &str = "/plan-my-trip/";
    path.strip_prefix(PREFIX)
}

fn generate_tag() -> String {
    generate(8, "abfksjdfhslkjfh")
}

fn fetch_itinerary(store: &Store, tag: &str) -> Option<String> {
    store.get(tag).ok().flatten().and_then(|value| str::from_utf8(&value).ok().map(|s| s.to_string()))
}

fn store_itinerary(store: &Store, tag: &str, itinerary: &str) {
    if store.set(tag, itinerary.as_bytes()).is_ok() {
        println!("Data saved successfully.");
    } else {
        println!("Failed to save data.");
    }
}

fn create_json_response(itinerary: &str, tag: &str) -> String {
    serde_json::to_string(&json!({
        "itinerary": itinerary,
        "tag": tag
    })).unwrap_or_else(|_| "{\"error\": \"Internal Server Error\"}".to_string())
}

#[http_component]
fn handle_travel_helper(req: Request) -> anyhow::Result<impl IntoResponse> {
    let store = Store::open_default()?;
    let path = req.path();
    
    if let Some(tag) = extract_tag(path) {
        if let Some(itinerary) = fetch_itinerary(&store, tag) {
            let json_response = create_json_response(&itinerary, tag);
            return Ok(Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(json_response)
                .build());
        } else {
            println!("No tag found: {:?}", tag);
        }
    }
    
    let request: SampleResponse = match serde_json::from_slice(req.body()) {
        Ok(request) => request,
        Err(_) => return Ok(Response::builder()
            .status(400)
            .header("content-type", "application/json")
            .body("{\"error\": \"Error while parsing request\"}")
            .build()),
    };

    let full_prompt = format!(
        "Create a summer vacation detailed itinerary trip to go to {} for a {}. {} people will be going on this trip planning to do {}",
        request.destination, request.duration, request.num_people, request.activities.join(", ")
    );

    let result_text = infer(InferencingModel::Llama2Chat, &full_prompt)
        .map(|res| res.text)
        .unwrap_or_else(|_| "Error in LLM".to_string());

    println!("Result text: {:?}", result_text);


    let itinerary_tag = generate_tag();
    store_itinerary(&store, &itinerary_tag, &result_text);

    let json_response = create_json_response(&result_text, &itinerary_tag);

    Ok(Response::builder()
        .status(201)
        .header("content-type", "application/json")
        .body(json_response)
        .build())
}
