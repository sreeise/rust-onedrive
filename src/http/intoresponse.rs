use crate::client::*;
use crate::http::{GraphResponse, UploadSessionClient};
use crate::types::delta::{Delta, NextLink};
use crate::types::{content::Content, delta::DeltaRequest};
use graph_error::{GraphFailure, GraphResult};
use reqwest::header::CONTENT_TYPE;
use std::cell::RefCell;
use std::convert::TryFrom;
use std::marker::PhantomData;
use std::sync::mpsc::{channel, Receiver};
use std::thread;

/// A trait for sending an API request and converting the response
/// to a suitable Rust type.
pub trait ToResponse {
    type Output;
    type SerdeJson;

    fn send(&self) -> Self::Output;
    fn value(&self) -> Self::SerdeJson;
}

pub struct IntoResponse<'a, I, T> {
    client: &'a Graph,
    ident: PhantomData<I>,
    phantom: PhantomData<T>,
    error: RefCell<Option<GraphFailure>>,
}

impl<'a, I, T> IntoResponse<'a, I, T> {
    pub fn new(client: &'a Graph) -> IntoResponse<I, T> {
        IntoResponse {
            client,
            ident: PhantomData,
            phantom: PhantomData,
            error: RefCell::new(None),
        }
    }

    pub(crate) fn new_error(client: &'a Graph, error: GraphFailure) -> IntoResponse<I, T> {
        IntoResponse {
            client,
            ident: PhantomData,
            phantom: PhantomData,
            error: RefCell::new(Some(error)),
        }
    }

    pub fn query(&self, key: &str, value: &str) -> &Self {
        self.client.builder().as_mut().append_query_pair(key, value);
        self
    }

    pub fn select(&self, value: &[&str]) -> &Self {
        self.client.builder().as_mut().select(value);
        self
    }

    pub fn expand(&self, value: &[&str]) -> &Self {
        self.client.builder().as_mut().expand(value);
        self
    }

    pub fn filter(&self, value: &[&str]) -> &Self {
        self.client.builder().as_mut().filter(value);
        self
    }

    pub fn order_by(&self, value: &[&str]) -> &Self {
        self.client.builder().as_mut().order_by(value);
        self
    }

    pub fn search(&self, value: &str) -> &Self {
        self.client.builder().as_mut().search(value);
        self
    }

    pub fn format(&self, value: &str) -> &Self {
        self.client.builder().as_mut().format(value);
        self
    }

    pub fn skip(&self, value: &str) -> &Self {
        self.client.builder().as_mut().skip(value);
        self
    }

    pub fn top(&self, value: &str) -> &Self {
        self.client.builder().as_mut().top(value);
        self
    }

    fn delta<U: 'static + Send + NextLink + Clone>(&self) -> Receiver<Delta<U>>
    where
        for<'de> U: serde::Deserialize<'de>,
    {
        let (sender, receiver) = channel();
        if self.error.borrow().is_some() {
            sender.send(Delta::Done(self.error.replace(None))).unwrap();
            return receiver;
        }

        let builder = self.client.take_builder();
        let response: GraphResult<GraphResponse<U>> = self.client.request().execute(builder);
        if let Err(err) = response {
            sender.send(Delta::Done(Some(err))).unwrap();
            return receiver;
        }

        let token = self.client.request().token().clone();
        let response = response.unwrap();
        let mut next_link = response.value().next_link();
        sender.send(Delta::Next(response)).unwrap();

        thread::spawn(move || {
            let client = reqwest::Client::new();
            while let Some(next) = next_link {
                let res = client
                    .get(next.as_str())
                    .header(CONTENT_TYPE, "application/json")
                    .bearer_auth(token.as_str())
                    .send()
                    .map_err(GraphFailure::from);

                if let Err(err) = res {
                    next_link = None;
                    sender.send(Delta::Done(Some(err))).unwrap();
                } else {
                    let mut response = res.unwrap();
                    if let Some(err) = GraphFailure::from_response(&mut response) {
                        next_link = None;
                        sender.send(Delta::Done(Some(err))).unwrap();
                    } else {
                        let value_res: GraphResult<U> = response.json().map_err(GraphFailure::from);
                        match value_res {
                            Ok(value) => {
                                next_link = value.next_link();
                                sender
                                    .send(Delta::Next(GraphResponse::new(response, value)))
                                    .unwrap();
                            },
                            Err(err) => {
                                next_link = None;
                                sender.send(Delta::Done(Some(err))).unwrap();
                            },
                        }
                    }
                }
            }
            sender.send(Delta::Done(None)).unwrap();
        });

        receiver
    }

    fn serde_json(&self) -> GraphResult<GraphResponse<serde_json::Value>> {
        if self.error.borrow().is_some() {
            return Err(self.error.replace(None).unwrap());
        }
        let mut response = self.client.request().response(self.client.take_builder())?;
        let value: serde_json::Value = response.json()?;
        Ok(GraphResponse::new(response, value))
    }

    pub fn json<U>(&self) -> GraphResult<U>
    where
        for<'de> U: serde::Deserialize<'de>,
    {
        if self.error.borrow().is_some() {
            return Err(self.error.replace(None).unwrap());
        }
        let mut response = self.client.request().response(self.client.take_builder())?;
        Ok(response.json()?)
    }
}

impl<'a, I, T> ToResponse for IntoResponse<'a, I, T>
where
    for<'de> T: serde::Deserialize<'de>,
{
    type Output = GraphResult<GraphResponse<T>>;
    type SerdeJson = GraphResult<GraphResponse<serde_json::Value>>;

    fn send(&self) -> Self::Output {
        if self.error.borrow().is_some() {
            return Err(self.error.replace(None).unwrap());
        }
        let builder = self.client.take_builder();
        self.client.request().execute(builder)
    }

    fn value(&self) -> Self::SerdeJson {
        self.serde_json()
    }
}

impl<'a, I> ToResponse for IntoResponse<'a, I, UploadSessionClient> {
    type Output = GraphResult<UploadSessionClient>;
    type SerdeJson = GraphResult<GraphResponse<serde_json::Value>>;

    fn send(&self) -> Self::Output {
        if self.error.borrow().is_some() {
            return Err(self.error.replace(None).unwrap());
        }
        self.client
            .request()
            .upload_session(self.client.take_builder())
    }

    fn value(&self) -> Self::SerdeJson {
        self.serde_json()
    }
}

impl<'a, I> ToResponse for IntoResponse<'a, I, GraphResponse<Content>> {
    type Output = GraphResult<GraphResponse<Content>>;
    type SerdeJson = GraphResult<GraphResponse<serde_json::Value>>;

    fn send(&self) -> Self::Output {
        if self.error.borrow().is_some() {
            return Err(self.error.replace(None).unwrap());
        }
        let builder = self.client.take_builder();
        let response = self.client.request().response(builder)?;
        Ok(GraphResponse::try_from(response)?)
    }

    fn value(&self) -> Self::SerdeJson {
        if self.error.borrow().is_some() {
            return Err(self.error.replace(None).unwrap());
        }
        let builder = self.client.take_builder();
        let mut response = self.client.request().response(builder)?;
        if let Ok(content) = response.text() {
            Ok(GraphResponse::new(
                response,
                serde_json::json!({ "content": content }),
            ))
        } else {
            Ok(GraphResponse::new(
                response,
                serde_json::json!({ "content": "" }),
            ))
        }
    }
}

impl<'a, I, T: 'static + Send + NextLink + Clone> ToResponse
    for IntoResponse<'a, I, DeltaRequest<T>>
where
    for<'de> T: serde::Deserialize<'de>,
{
    type Output = Receiver<Delta<T>>;
    type SerdeJson = Receiver<Delta<serde_json::Value>>;

    fn send(&self) -> Self::Output {
        self.delta::<T>()
    }

    fn value(&self) -> Self::SerdeJson {
        self.delta::<serde_json::Value>()
    }
}
