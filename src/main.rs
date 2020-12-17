use warp::Filter;

type Result<T> = std::result::Result<T, warp::Rejection>;

mod error {
    use serde_derive::Serialize;
    use std::convert::Infallible;
    use thiserror::Error;
    use warp::http::StatusCode;
    use warp::reject::Rejection;
    use warp::reply::Reply;

    #[derive(Error, Debug)]
    pub enum Error {
        #[error("error parsing yaml: {0}")]
        DroneYamlError(#[from] serde_yaml::Error),
    }

    impl warp::reject::Reject for Error {}

    #[derive(Serialize)]
    struct ErrorResponse {
        message: String,
    }

    pub async fn handle_rejection(err: Rejection) -> std::result::Result<impl Reply, Infallible> {
        let code;
        let message;

        if err.is_not_found() {
            code = StatusCode::NOT_FOUND;
            message = "Not Found";
        } else if let Some(_) = err.find::<warp::filters::body::BodyDeserializeError>() {
            code = StatusCode::BAD_REQUEST;
            message = "Invalid Body";
        } else if let Some(e) = err.find::<Error>() {
            match e {
                _ => {
                    eprintln!("unhandled application error: {:?}", err);
                    code = StatusCode::INTERNAL_SERVER_ERROR;
                    message = "Internal Server Error";
                }
            }
        } else if let Some(_) = err.find::<warp::reject::MethodNotAllowed>() {
            code = StatusCode::METHOD_NOT_ALLOWED;
            message = "Method Not Allowed";
        } else {
            eprintln!("unhandled error: {:?}", err);
            code = StatusCode::INTERNAL_SERVER_ERROR;
            message = "Internal Server Error";
        }

        let json = warp::reply::json(&ErrorResponse {
            message: message.into(),
        });

        Ok(warp::reply::with_status(json, code))
    }
}

mod drone {
    use serde_derive::{Deserialize, Serialize};
    #[derive(Deserialize, Serialize)]
    pub struct Config {
        pub data: String,
    }

    #[derive(Deserialize, Serialize)]
    pub struct Request {
        pub config: Config,
        // Drone actually sends more, which we freely ignore for now.
    }

    #[derive(Deserialize, Serialize, Clone)]
    pub struct Pipeline {
        // We kinda expect drone > 0.8 syntax, which introduces this field.
        // If it is not present in the data but specified here, serde fails to deserialize,
        // and drone-riot-convert just passes through the yaml document unchanged.
        pub kind: String,

        // this field will get changed to "<name>-<instance-number>"
        pub name: String,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub parallelism: Option<usize>,

        #[serde(rename = "type")]
        type_: String,

        // capture all other data
        #[serde(flatten)]
        extra: indexmap::IndexMap<String, serde_yaml::Value>,
    }
}

async fn convert_handler(request: drone::Request) -> Result<impl warp::reply::Reply> {
    const PARALLELISM_MAX: usize = 64;
    println!("drone-riot-conv: handling request");

    let mut result = String::new();
    for (n, doc) in request.config.data.split("\n---\n").enumerate() {
        let parsed: drone::Pipeline = match serde_yaml::from_str(doc) {
            Ok(val) => val,
            Err(e) => {
                println!(
                    "drone-riot-conv: warning: error parsing yaml document {}: {}. passing through.",
                    e,
                    n + 1
                );
                result += doc;
                continue;
            }
        };

        if let Some(mut value) = parsed.parallelism {
            if value > PARALLELISM_MAX {
                println!(
                    "convert_handler: limiting parallelism value to {}",
                    PARALLELISM_MAX
                );
                value = PARALLELISM_MAX;
            }
            for n in 1..=value {
                let mut instance = parsed.clone();
                instance.name += &format!("-{}", n);
                instance.parallelism = None;
                result += &serde_yaml::to_string(&instance)
                    .map_err(|e| warp::reject::custom(error::Error::DroneYamlError(e)))?;
                result += "\n";
            }
        } else {
            result += doc;
        }
    }

    Ok(warp::reply::json(&drone::Config { data: result }))
}

#[tokio::main]
async fn main() {
    println!("drone-riot-conv: started");
    let convert = warp::post()
        .and(warp::path("convert"))
        .and(warp::body::json())
        .and_then(convert_handler)
        .recover(error::handle_rejection);

    warp::serve(convert).run(([127, 0, 0, 1], 3030)).await;
}
