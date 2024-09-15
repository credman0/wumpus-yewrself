use wasm_bindgen_futures::JsFuture;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;
use reqwest::Client;
use reqwest::header;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::window;
use rand::seq::SliceRandom;
use base64;

fn generate_csv(pool: &[Card]) -> String {
    let mut csv = String::new();
    csv.push_str("name\n");
    for card in pool {
        // let rarity_letter = card.rarity.as_ref().map(|r| r.chars().next()).flatten().unwrap_or_default();
        csv.push_str(&format!("\"{}\"\n", card.name));
    }
    csv
}

fn download_csv(cube_name:&String, pool: &[Card]) {
    let csv_data = generate_csv(pool);
    let encoded_data = base64::encode(csv_data);
    let link = format!("data:text/csv;base64,{}", encoded_data);
    
    // Create a temporary anchor element to trigger the download
    let document = web_sys::window().unwrap().document().unwrap();
    let anchor = document.create_element("a").unwrap();
    
    anchor.set_attribute("href", &link).unwrap();
    anchor.set_attribute("download", &format!("{}.csv", cube_name)).unwrap();

    let anchor = anchor.dyn_into::<web_sys::HtmlElement>().unwrap();
    
    // Trigger click on the anchor to download the file
    anchor.click();

    // Remove the anchor from the document
    anchor.remove();
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Card {
    name: String,
    rarity: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ApiResponse {
    object: String,
    total_cards: u32,
    has_more: bool,
    next_page: Option<String>,
    data: Vec<Card>,
}

async fn sleep(ms: u32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let window = window().unwrap();
        window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms as i32).unwrap();
    });
    JsFuture::from(promise).await.unwrap();
}

#[function_component(App)]
fn app() -> Html {
    let cube_name = use_state(|| "pool".to_string());
    let without_rarity = use_state(|| false);
    let sets = use_state(|| "akh,dom,war,stx,znr".to_string());
    let cube = use_state(Vec::new);
    
    // States for pool generation
    let num_packs = use_state(|| 18);
    let num_r = use_state(|| 1);
    let num_u = use_state(|| 3);
    let num_c = use_state(|| 10);
    let generated_pool = use_state(Vec::new);


    let download_pool = {
        let generated_pool = generated_pool.clone();
        let cube_name = cube_name.clone();

        Callback::from(move |_| {
            download_csv(&*cube_name, &*generated_pool);
        })
    };

    let fetch_cube = {
        let cube = cube.clone();
        let without_rarity = *without_rarity;
        let sets = sets.clone();

        Callback::from(move |_| {
            let cube = cube.clone();
            let sets = sets.clone();
            let without_rarity = without_rarity;

            spawn_local(async move {
                let mut headers = header::HeaderMap::new();

                let client = Client::builder()
                    .default_headers(headers)
                    .build()
                    .unwrap();

                let sets: Vec<&str> = sets.split(',').collect();
                let query = format!("-t%3ABasic+AND+game%3Apaper+AND+({})",
                    sets.iter().map(|s| format!("set%3A{}", s)).collect::<Vec<_>>().join("+OR+"));
                let base_url = format!("https://api.scryfall.com/cards/search?order=name&format=json&q=({})&page=1", query);

                let mut new_cube = Vec::new();
                let mut url = base_url.clone();

                while let Ok((response, next_page_url)) = fetch_page(&client, &url).await {
                    if response.status().is_success() {
                        let json_response = response.text().await.unwrap();
                        let json_response: ApiResponse = serde_json::from_str(&json_response).unwrap();
                        let with_rarity = !without_rarity;
                        new_cube.extend(process_json(&json_response.data, with_rarity));
                        if let Some(next_page) = json_response.next_page {
                            url = next_page;
                            sleep(100).await;
                        } else {
                            break;
                        }
                    } else {
                        log::error!("Request failed with status: {}", response.status());
                        break;
                    }
                }

                cube.set(new_cube);
            });
        })
    };

    let generate_pool = {
        let cube = cube.clone();
        let num_packs = *num_packs;
        let num_r = *num_r;
        let num_u = *num_u;
        let num_c = *num_c;
        let generated_pool = generated_pool.clone();

        Callback::from(move |_| {
            let mut r = Vec::new();
            let mut u = Vec::new();
            let mut c = Vec::new();

            for card in &*cube {
                match card.rarity.as_deref() {
                    Some("rare") | Some("mythic") => r.push(card.clone()),
                    Some("uncommon") => u.push(card.clone()),
                    Some("common") => c.push(card.clone()),
                    _ => (),
                }
            }
            let mut cards = Vec::new();
            cards.extend(r.choose_multiple(&mut rand::thread_rng(), num_r * num_packs).cloned());
            cards.extend(u.choose_multiple(&mut rand::thread_rng(), num_u * num_packs).cloned());
            cards.extend(c.choose_multiple(&mut rand::thread_rng(), num_c * num_packs).cloned());

            generated_pool.set(cards);
        })
    };

    html! {
        <div class="container">
			<div class="form-group">
				<label for="sets">{"Sets"}</label>
				<input type="text" id="sets" value={sets.to_string()} 
				oninput={Callback::from(move |e: InputEvent| sets.set(e.target().unwrap_throw().dyn_into().map(|x: HtmlInputElement| x.value()).unwrap_or_default()))} 
				placeholder="Enter sets" />
                       
                <label for="without-rarity">{"Without Rarity"}</label>
                <input type="checkbox" id="without-rarity" checked={*without_rarity} 
                       onclick={Callback::from(move |_| without_rarity.set(!*without_rarity))} />
                       
                <button onclick={fetch_cube}>{"Fetch Cube"}</button>
            </div>
    
            <div class="cube-list">
                { for cube.iter().map(|card| html! { <p>{ format!("{:?}", card) }</p> }) }
            </div>
    
			<div class="form-group">
				<label for="num-packs">{"Number of Packs"}</label>
				<input type="number" id="num-packs" value={num_packs.to_string()} 
					   oninput={Callback::from(move |e: InputEvent| num_packs.set(e.target().unwrap_throw().dyn_into().map(|x: HtmlInputElement| x.value().parse().unwrap_throw()).unwrap_or(18)))} 
					   placeholder="Enter number of packs" />
			
				<label for="num-r">{"Number of Rares per Pack"}</label>
				<input type="number" id="num-r" value={num_r.to_string()} 
					   oninput={Callback::from(move |e: InputEvent| num_r.set(e.target().unwrap_throw().dyn_into().map(|x: HtmlInputElement| x.value().parse().unwrap_or(0)).unwrap_or(0)))} 
					   placeholder="Enter number of rares" />
					   
				<label for="num-u">{"Number of Uncommons per Pack"}</label>
				<input type="number" id="num-u" value={num_u.to_string()} 
					   oninput={Callback::from(move |e: InputEvent| num_u.set(e.target().unwrap_throw().dyn_into().map(|x: HtmlInputElement| x.value().parse().unwrap_or(0)).unwrap_or(0)))} 
					   placeholder="Enter number of uncommons" />
					   
				<label for="num-c">{"Number of Commons per Pack"}</label>
				<input type="number" id="num-c" value={num_c.to_string()} 
					   oninput={Callback::from(move |e: InputEvent| num_c.set(e.target().unwrap_throw().dyn_into().map(|x: HtmlInputElement| x.value().parse().unwrap_or(10)).unwrap_or(10)))} 
					   placeholder="Enter number of commons" />
			
				<button onclick={generate_pool}>{"Generate Pool"}</button>
			</div>
    
            <div class="cube-list">
                { for generated_pool.iter().map(|card| html! { <p>{ format!("{:?}", card) }</p> }) }
            </div>
            <div class="form-group">
                <label for="cube-name">{"Pool Name"}</label>
                <input type="text" id="cube-name" value={(*cube_name).clone()} 
                    oninput={Callback::from(move |e: InputEvent| cube_name.set(e.to_string().into()))} 
                    placeholder="Enter pool name" />
            </div>
            <button onclick={download_pool}>{"Download CSV"}</button>
        </div>
    }
    
    
}

async fn fetch_page(client: &Client, url: &str) -> Result<(reqwest::Response, Option<String>), reqwest::Error> {
    let response = client.get(url).send().await?;
    let next_page_url = response.headers().get("X-Scryfall-Next-Page")
        .map(|v| v.to_str().unwrap_or("").to_string());
    Ok((response, next_page_url))
}

fn process_json(data: &[Card], with_rarity: bool) -> Vec<Card> {
    data.iter().map(|card| {
        Card {
            name: card.name.clone(),
            rarity: if with_rarity { card.rarity.clone() } else { None },
        }
    }).collect()
}

fn main() {
    yew::Renderer::<App>::new().render();
}
