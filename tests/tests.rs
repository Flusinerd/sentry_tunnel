#[cfg(test)]
mod tests {
    use sentry_tunnel::config::Host;
    use gotham::hyper::http::{header, HeaderValue, StatusCode};
    use gotham::test::TestServer;

    use httpmock::prelude::*;
    use mime::Mime;
    use sentry_tunnel::config::Config;
    use sentry_tunnel::envelope::BodyError;
    use sentry_tunnel::server::{router, HeaderError};

    #[test]
    fn test_correct_behaviour() {
        let server = MockServer::start();
        let sentry_mock = server.mock(|when, then| {
            when.method(POST).path("/api/5/envelope/");
            then.status(200);
        });
        let test_config = Config {
            remote_hosts: Config::clean_remote_hosts(&[server.url("")]),
            project_ids: vec!["5".to_string()],
            port: 7878,
            tunnel_path: "/tunnel".to_string(),
            ip: "0.0.0.0".to_string(),
        };
        let test_server = TestServer::new(router(
            &test_config.tunnel_path.clone(),
            test_config.clone(),
        ))
        .unwrap();
        let json = r#"{"sent_at":"2021-10-14T17:10:40.136Z","sdk":{"name":"sentry.javascript.browser","version":"6.13.3"},"dsn":"http://public@HOST_TEST_REPLACE/5"}
        {"type":"session"}
        {"sid":"751d80dc94e34cd282a2cf1fe698a8d2","init":true,"started":"2021-10-14T17:10:40.135Z","timestamp":"2021-10-14T17:10:40.135Z","status":"ok","errors":0,"attrs":{"release":"test_project@1.0"}"#;
        let json = json
            .replace("HOST_TEST_REPLACE", &server.address().to_string())
            .to_owned();
        println!("{:?}", json);
        let mime = "application/json".parse::<Mime>().unwrap();
        let response = test_server
            .client()
            .post(
                "http://localhost".to_owned() + &test_config.tunnel_path,
                json.clone(),
                mime,
            )
            .with_header(
                header::CONTENT_LENGTH,
                HeaderValue::from_str(&format!("{}", json.as_bytes().len())).unwrap(),
            )
            .perform()
            .unwrap();

        sentry_mock.assert();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_session_replay_envelope() {
        let server = MockServer::start();
        let sentry_mock = server.mock(|when, then| {
            when.method(POST).path("/api/6/envelope/");
            then.status(200);
        });
        let test_config = Config {
            remote_hosts: Config::clean_remote_hosts(&[server.url("")]),
            project_ids: vec!["6".to_string()],
            port: 7878,
            tunnel_path: "/tunnel".to_string(),
            ip: "0.0.0.0".to_string(),
        };
        let test_server = TestServer::new(router(
            &test_config.tunnel_path.clone(),
            test_config.clone(),
        ))
        .unwrap();
        
        // Session replay envelope with multiple lines (header + multiple items)
        let json = r#"{"event_id":"65de0c6c634c4b29b63eb2af58e7bfa7","sent_at":"2025-07-09T21:52:36.839Z","sdk":{"name":"sentry.javascript.react","version":"9.24.0"},"dsn":"http://public@HOST_TEST_REPLACE/6"}
{"type":"replay_event"}
{"type":"replay_event","replay_start_timestamp":1752097947.846,"timestamp":1752097956.838,"error_ids":["a11c57d12066461982ff3fbb78ab0752"],"trace_ids":["836b56305ed84493a72b4a4f58cba356","c8fd251f22884313a09208497f1f3753"],"urls":["https://my.langguth.com/shop/customers/4285ff71-028d-4755-850c-090f520695b8/machines/ec19bb09-374e-4ace-ab26-2ae246f82ce9"],"replay_id":"65de0c6c634c4b29b63eb2af58e7bfa7","segment_id":0,"replay_type":"buffer","request":{"url":"https://my.langguth.com/shop/customers/4285ff71-028d-4755-850c-090f520695b8/machines/ec19bb09-374e-4ace-ab26-2ae246f82ce9","headers":{"User-Agent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36"}},"event_id":"65de0c6c634c4b29b63eb2af58e7bfa7","environment":"production","release":"1.9.1","sdk":{"integrations":["InboundFilters","FunctionToString","BrowserApiErrors","Breadcrumbs","GlobalHandlers","LinkedErrors","Dedupe","HttpContext","BrowserSession","BrowserTracing","Replay","RewriteFrames"],"name":"sentry.javascript.react","version":"9.24.0"},"contexts":{"react":{"version":"19.0.0"}},"transaction":"/customers/$customerId/machines/$machineId","user":{"ip_address":"{{auto}}"},"platform":"javascript"}
{"type":"replay_recording","length":57959}
{"segment_id":0}
binary_data_placeholder"#;
        
        let json = json
            .replace("HOST_TEST_REPLACE", &server.address().to_string())
            .to_owned();
        println!("{:?}", json);
        
        let mime = "application/json".parse::<Mime>().unwrap();
        let response = test_server
            .client()
            .post(
                "http://localhost".to_owned() + &test_config.tunnel_path,
                json.clone(),
                mime,
            )
            .with_header(
                header::CONTENT_LENGTH,
                HeaderValue::from_str(&format!("{}", json.as_bytes().len())).unwrap(),
            )
            .perform()
            .unwrap();

        sentry_mock.assert();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_invalid_project_id() {
        let test_config = Config {
            remote_hosts: vec![Host("https://sentry.example.com/".to_string())],
            project_ids: vec!["5".to_string()],
            port: 7878,
            tunnel_path: "/tunnel".to_string(),
            ip: "0.0.0.0".to_string(),
        };
        let test_server = TestServer::new(router(
            &test_config.tunnel_path.clone(),
            test_config.clone(),
        ))
        .unwrap();
        let json = r#"{"sent_at":"2021-10-14T17:10:40.136Z","sdk":{"name":"sentry.javascript.browser","version":"6.13.3"},"dsn":"https://public@sentry.example.com/4"}
        {"type":"session"}
        {"sid":"751d80dc94e34cd282a2cf1fe698a8d2","init":true,"started":"2021-10-14T17:10:40.135Z","timestamp":"2021-10-14T17:10:40.135Z","status":"ok","errors":0,"attrs":{"release":"test_project@1.0"}"#;
        let mime = "application/json".parse::<Mime>().unwrap();
        let response = test_server
            .client()
            .post(
                "http://localhost".to_owned() + &test_config.tunnel_path,
                json,
                mime,
            )
            .with_header(
                header::CONTENT_LENGTH,
                HeaderValue::from_str(&format!("{}", json.as_bytes().len())).unwrap(),
            )
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = response.read_body().unwrap();
        let expc = format!("{}", BodyError::InvalidProjectId);

        assert_eq!(String::from_utf8(body).unwrap(), expc);
    }

    #[test]
    fn test_missing_dsn() {
        let test_config = Config {
            remote_hosts: vec![Host("https://sentry.example.com/".to_string())],
            project_ids: vec!["5".to_string()],
            port: 7878,
            tunnel_path: "/tunnel".to_string(),
            ip: "0.0.0.0".to_string(),
        };
        let test_server = TestServer::new(router(
            &test_config.tunnel_path.clone(),
            test_config.clone(),
        ))
        .unwrap();
        let json = r#"{"sent_at":"2021-10-14T17:10:40.136Z","sdk":{"name":"sentry.javascript.browser","version":"6.13.3"}}
        {"type":"session"}
        {"sid":"751d80dc94e34cd282a2cf1fe698a8d2","init":true,"started":"2021-10-14T17:10:40.135Z","timestamp":"2021-10-14T17:10:40.135Z","status":"ok","errors":0,"attrs":{"release":"test_project@1.0"}"#;
        let mime = "application/json".parse::<Mime>().unwrap();
        let response = test_server
            .client()
            .post(
                "http://localhost".to_owned() + &test_config.tunnel_path,
                json,
                mime,
            )
            .with_header(
                header::CONTENT_LENGTH,
                HeaderValue::from_str(&format!("{}", json.as_bytes().len())).unwrap(),
            )
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = response.read_body().unwrap();
        let expc = format!("{}", BodyError::MissingDsnKeyInHeader);

        assert_eq!(String::from_utf8(body).unwrap(), expc);
    }

    #[test]
    fn test_dsn_host_invalid() {
        let test_config = Config {
            remote_hosts: vec![Host("https://sentry.example.com/".to_string())],
            project_ids: vec!["5".to_string()],
            port: 7878,
            tunnel_path: "/tunnel".to_string(),
            ip: "0.0.0.0".to_string(),
        };
        let test_server = TestServer::new(router(
            &test_config.tunnel_path.clone(),
            test_config.clone(),
        ))
        .unwrap();
        let json = r#"{"sent_at":"2021-10-14T17:10:40.136Z","sdk":{"name":"sentry.javascript.browser","version":"6.13.3"},"dsn":"https://public@not_a_valid_host.example.com/5"}
        {"type":"session"}
        {"sid":"751d80dc94e34cd282a2cf1fe698a8d2","init":true,"started":"2021-10-14T17:10:40.135Z","timestamp":"2021-10-14T17:10:40.135Z","status":"ok","errors":0,"attrs":{"release":"test_project@1.0"}"#;
        let mime = "application/json".parse::<Mime>().unwrap();
        let response = test_server
            .client()
            .post(
                "http://localhost".to_owned() + &test_config.tunnel_path,
                json,
                mime,
            )
            .with_header(
                header::CONTENT_LENGTH,
                HeaderValue::from_str(&format!("{}", json.as_bytes().len())).unwrap(),
            )
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = response.read_body().unwrap();
        let expc = format!("{}", HeaderError::InvalidHost);

        assert_eq!(String::from_utf8(body).unwrap(), expc);
    }

    #[test]
    fn test_empty_body() {
        let test_config = Config {
            remote_hosts: vec![Host("https://sentry.example.com/".to_string())],
            project_ids: vec!["5".to_string()],
            port: 7878,
            tunnel_path: "/tunnel".to_string(),
            ip: "0.0.0.0".to_string(),
        };
        let test_server = TestServer::new(router(
            &test_config.tunnel_path.clone(),
            test_config.clone(),
        ))
        .unwrap();
        let json = "";
        let mime = "application/json".parse::<Mime>().unwrap();
        let response = test_server
            .client()
            .post(
                "http://localhost".to_owned() + &test_config.tunnel_path,
                json,
                mime,
            )
            .with_header(
                header::CONTENT_LENGTH,
                HeaderValue::from_str(&format!("{}", json.as_bytes().len())).unwrap(),
            )
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = response.read_body().unwrap();
        let expc = format!("{}", BodyError::EmptyBody);

        assert_eq!(String::from_utf8(body).unwrap(), expc);
    }

    #[test]
    fn test_insufficient_lines() {
        let test_config = Config {
            remote_hosts: vec![Host("https://sentry.example.com/".to_string())],
            project_ids: vec!["5".to_string()],
            port: 7878,
            tunnel_path: "/tunnel".to_string(),
            ip: "0.0.0.0".to_string(),
        };
        let test_server = TestServer::new(router(
            &test_config.tunnel_path.clone(),
            test_config.clone(),
        ))
        .unwrap();
        let json = r#"{"sent_at":"2021-10-14T17:10:40.136Z","sdk":{"name":"sentry.javascript.browser","version":"6.13.3"},"dsn":"https://public@sentry.example.com/5"}"#;
        let mime = "application/json".parse::<Mime>().unwrap();
        let response = test_server
            .client()
            .post(
                "http://localhost".to_owned() + &test_config.tunnel_path,
                json,
                mime,
            )
            .with_header(
                header::CONTENT_LENGTH,
                HeaderValue::from_str(&format!("{}", json.as_bytes().len())).unwrap(),
            )
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = response.read_body().unwrap();
        let expc = format!("{}", BodyError::InvalidNumberOfLines);

        assert_eq!(String::from_utf8(body).unwrap(), expc);
    }
    
    #[test]
    fn test_big_envelope() {
        let server = MockServer::start();
        let sentry_mock = server.mock(|when, then| {
            when.method(POST).path("/api/5/envelope/");
            then.status(200);
        });
        let test_config = Config {
            remote_hosts: Config::clean_remote_hosts(&[server.url("")]),
            project_ids: vec!["5".to_string()],
            port: 7878,
            tunnel_path: "/tunnel".to_string(),
            ip: "0.0.0.0".to_string(),
        };
        let test_server = TestServer::new(router(
            &test_config.tunnel_path.clone(),
            test_config.clone(),
        ))
        .unwrap();
        let json = r#"{"event_id":"85ed182e014747aa917583711139a6fe","sent_at":"2021-11-04T13:25:26.636Z","sdk":{"name":"sentry.javascript.react","version":"6.13.3"},"dsn":"http://public@HOST_TEST_REPLACE/5"}
{"type":"event","sample_rates":[{}]}
{"exception":{"values":[{"type":"Error","value":"Super test","mechanism":{"handled":true,"type":"generic"}}]},"level":"error","event_id":"85ed182e014747aa917583711139a6fe","platform":"javascript","timestamp":1636032326.628,"environment":"prod","release":"GeoCRUD-front@1.0","sdk":{"integrations":["InboundFilters","FunctionToString","TryCatch","Breadcrumbs","GlobalHandlers","LinkedErrors","Dedupe","UserAgent","BrowserTracing"],"name":"sentry.javascript.react","version":"6.13.3","packages":[{"name":"npm:@sentry/react","version":"6.13.3"}]},"breadcrumbs":[{"timestamp":1636032321.638,"category":"fetch","data":{"method":"GET","url":"/api/crud/settings/","__span":"8d0fbc950957efa7","status_code":200},"type":"http"},{"timestamp":1636032322.128,"category":"fetch","data":{"method":"GET","url":"https://api.mapbox.com/styles/v1/makinacorpus/cktwqn3220g7618moe1oyxbpv?access_token=pk.eyJ1IjoibWFraW5hY29ycHVzIiwiYSI6ImNrMXMwNjd0MDBhNGIzZm51YTQ1djVqazMifQ.5TluOfrnGyfiExCCrJXV3A","status_code":200},"type":"http"},{"timestamp":1636032322.458,"category":"fetch","data":{"method":"GET","url":"https://api.mapbox.com/v4/mapbox.mapbox-streets-v8,mapbox.mapbox-terrain-v2.json?secure&access_token=pk.eyJ1IjoibWFraW5hY29ycHVzIiwiYSI6ImNrMXMwNjd0MDBhNGIzZm51YTQ1djVqazMifQ.5TluOfrnGyfiExCCrJXV3A","status_code":200},"type":"http"},{"timestamp":1636032322.461,"category":"fetch","data":{"method":"GET","url":"https://api.mapbox.com/styles/v1/makinacorpus/cktwqn3220g7618moe1oyxbpv/aooqdcrzrpe6ncdm1ykzofhaw/sprite.json?access_token=pk.eyJ1IjoibWFraW5hY29ycHVzIiwiYSI6ImNrMXMwNjd0MDBhNGIzZm51YTQ1djVqazMifQ.5TluOfrnGyfiExCCrJXV3A","status_code":200},"type":"http"},{"timestamp":1636032322.463,"category":"fetch","data":{"method":"GET","url":"https://api.mapbox.com/styles/v1/makinacorpus/cktwqn3220g7618moe1oyxbpv/aooqdcrzrpe6ncdm1ykzofhaw/sprite.png?access_token=pk.eyJ1IjoibWFraW5hY29ycHVzIiwiYSI6ImNrMXMwNjd0MDBhNGIzZm51YTQ1djVqazMifQ.5TluOfrnGyfiExCCrJXV3A","status_code":200},"type":"http"},{"timestamp":1636032322.505,"category":"fetch","data":{"method":"POST","url":"https://events.mapbox.com/events/v2?access_token=pk.eyJ1IjoibWFraW5hY29ycHVzIiwiYSI6ImNrMXMwNjd0MDBhNGIzZm51YTQ1djVqazMifQ.5TluOfrnGyfiExCCrJXV3A"},"level":"error","type":"http"},{"timestamp":1636032322.506,"category":"fetch","data":{"method":"POST","url":"https://events.mapbox.com/events/v2?access_token=pk.eyJ1IjoibWFraW5hY29ycHVzIiwiYSI6ImNrMXMwNjd0MDBhNGIzZm51YTQ1djVqazMifQ.5TluOfrnGyfiExCCrJXV3A"},"level":"error","type":"http"},{"timestamp":1636032322.696,"category":"fetch","data":{"method":"GET","url":"/api/crud/layers/8/features/?ordering=&page=1&page_size=50&search=","status_code":200},"type":"http"},{"timestamp":1636032322.697,"category":"fetch","data":{"method":"GET","url":"https://geocompostelle.makina-corpus.net/api/crud/layers/8/tilejson/","status_code":200},"type":"http"},{"timestamp":1636032322.903,"category":"fetch","data":{"method":"GET","url":"https://api.mapbox.com/fonts/v1/mapbox/DIN%20Pro%20Medium,Arial%20Unicode%20MS%20Regular/8192-8447.pbf?access_token=pk.eyJ1IjoibWFraW5hY29ycHVzIiwiYSI6ImNrMXMwNjd0MDBhNGIzZm51YTQ1djVqazMifQ.5TluOfrnGyfiExCCrJXV3A","status_code":200},"type":"http"},{"timestamp":1636032322.903,"category":"fetch","data":{"method":"GET","url":"https://api.mapbox.com/fonts/v1/mapbox/DIN%20Pro%20Medium,Arial%20Unicode%20MS%20Regular/0-255.pbf?access_token=pk.eyJ1IjoibWFraW5hY29ycHVzIiwiYSI6ImNrMXMwNjd0MDBhNGIzZm51YTQ1djVqazMifQ.5TluOfrnGyfiExCCrJXV3A","status_code":200},"type":"http"},{"timestamp":1636032322.903,"category":"fetch","data":{"method":"GET","url":"https://api.mapbox.com/fonts/v1/mapbox/DIN%20Pro%20Regular,Arial%20Unicode%20MS%20Regular/0-255.pbf?access_token=pk.eyJ1IjoibWFraW5hY29ycHVzIiwiYSI6ImNrMXMwNjd0MDBhNGIzZm51YTQ1djVqazMifQ.5TluOfrnGyfiExCCrJXV3A","status_code":200},"type":"http"},{"timestamp":1636032322.931,"category":"fetch","data":{"method":"GET","url":"https://api.mapbox.com/fonts/v1/mapbox/DIN%20Pro%20Bold,Arial%20Unicode%20MS%20Bold/0-255.pbf?access_token=pk.eyJ1IjoibWFraW5hY29ycHVzIiwiYSI6ImNrMXMwNjd0MDBhNGIzZm51YTQ1djVqazMifQ.5TluOfrnGyfiExCCrJXV3A","status_code":200},"type":"http"},{"timestamp":1636032322.931,"category":"fetch","data":{"method":"GET","url":"https://api.mapbox.com/fonts/v1/mapbox/DIN%20Pro%20Italic,Arial%20Unicode%20MS%20Regular/0-255.pbf?access_token=pk.eyJ1IjoibWFraW5hY29ycHVzIiwiYSI6ImNrMXMwNjd0MDBhNGIzZm51YTQ1djVqazMifQ.5TluOfrnGyfiExCCrJXV3A","status_code":200},"type":"http"},{"timestamp":1636032322.977,"category":"fetch","data":{"method":"GET","url":"https://api.mapbox.com/fonts/v1/mapbox/DIN%20Pro%20Regular,Arial%20Unicode%20MS%20Regular/512-767.pbf?access_token=pk.eyJ1IjoibWFraW5hY29ycHVzIiwiYSI6ImNrMXMwNjd0MDBhNGIzZm51YTQ1djVqazMifQ.5TluOfrnGyfiExCCrJXV3A","status_code":200},"type":"http"},{"timestamp":1636032322.978,"category":"fetch","data":{"method":"GET","url":"https://api.mapbox.com/fonts/v1/mapbox/DIN%20Pro%20Regular,Arial%20Unicode%20MS%20Regular/256-511.pbf?access_token=pk.eyJ1IjoibWFraW5hY29ycHVzIiwiYSI6ImNrMXMwNjd0MDBhNGIzZm51YTQ1djVqazMifQ.5TluOfrnGyfiExCCrJXV3A","status_code":200},"type":"http"}],"request":{"url":"https://geocompostelle.makina-corpus.net/map/monuments","headers":{"User-Agent":"Mozilla/5.0 (X11; Linux x86_64; rv:94.0) Gecko/20100101 Firefox/94.0"}}}"#;
        let json = json
            .replace("HOST_TEST_REPLACE", &server.address().to_string())
            .to_owned();
        println!("{:?}", json);
        let mime = "application/json".parse::<Mime>().unwrap();
        let response = test_server
            .client()
            .post(
                "http://localhost".to_owned() + &test_config.tunnel_path,
                json.clone(),
                mime,
            )
            .with_header(
                header::CONTENT_LENGTH,
                HeaderValue::from_str(&format!("{}", json.as_bytes().len())).unwrap(),
            )
            .perform()
            .unwrap();

        sentry_mock.assert();
        assert_eq!(response.status(), StatusCode::OK);
    
    }
}
