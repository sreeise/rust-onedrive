use crate::client::Graph;
use crate::http::{GraphResponse, IntoResponse};
use crate::types::content::Content;
use reqwest::Method;

register_client!(ActivitiesRequest,);
register_client!(ActivitiesHistoryItemsRequest,);

impl<'a, Client> ActivitiesRequest<'a, Client>
where
    Client: crate::http::RequestClient,
{
    pub fn history_items(&self) -> ActivitiesHistoryItemsRequest<'a, Client> {
        ActivitiesHistoryItemsRequest::new(&self.client)
    }
}

impl<'a, Client> ActivitiesRequest<'a, Client>
where
    Client: crate::http::RequestClient,
{
    get!( list_activities, serde_json::Value => "/activities");
    post!([ create_activities, serde_json::Value => "/activities" ]);
    get!(| get_activities, serde_json::Value => "/activities/{{id}}");
    patch!([| update_activities, serde_json::Value => "/activities/{{id}}" ]);
    delete!(| remove_activities, GraphResponse<Content> => "/activities/{{id}}" );
}

impl<'a, Client> ActivitiesHistoryItemsRequest<'a, Client>
where
    Client: crate::http::RequestClient,
{
    get!(|| get_activity, serde_json::Value => "/activities/{{id}}/historyItems/{{id2}}/activity");
    get!(| list_history_items, serde_json::Value => "/activities/{{id}}/historyItems");
    post!([| create_history_items, serde_json::Value => "/activities/{{id}}/historyItems" ]);
    get!(|| get_history_items, serde_json::Value => "/activities/{{id}}/historyItems/{{id2}}");
    patch!([|| update_history_items, serde_json::Value => "/activities/{{id}}/historyItems/{{id2}}" ]);
    delete!(|| remove_history_items, GraphResponse<Content> => "/activities/{{id}}/historyItems/{{id2}}" );
}
