use async_std::sync::Mutex;
use crabrolls::prelude::*;
use serde::{Deserialize, Serialize};
use std::{error::Error, sync::Arc};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Post {
    id: u64,
    title: String,
    content: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind", content = "payload")]
enum Input {
    AddPost {
        title: String,
        content: String,
    },
    UpdatePost {
        id: u64,
        title: Option<String>,
        content: Option<String>,
    },
    DeletePost {
        id: u64,
    },
}

struct BlogApp {
    posts: Vec<Post>,
    next_id: u64,
}

impl BlogApp {
    fn new() -> Self {
        Self {
            posts: Vec::new(),
            next_id: 1,
        }
    }

    fn handle_add_post(&mut self, title: String, content: String) -> Result<(), Box<dyn Error>> {
        let id = self.next_id;
        self.next_id += 1;
        self.posts.push(Post { id, title, content });
        println!(
            "Added post {:?}",
            self.posts.last().expect("Failed to get last post")
        );
        Ok(())
    }

    fn handle_update_post(
        &mut self,
        id: u64,
        title: Option<String>,
        content: Option<String>,
    ) -> Result<(), Box<dyn Error>> {
        let post = self
            .posts
            .iter_mut()
            .find(|post| post.id == id)
            .ok_or("Post not found")?;
        if let Some(title) = title {
            post.title = title;
        }
        if let Some(content) = content {
            post.content = content;
        }

        println!("Updated post {:?}", post);
        Ok(())
    }

    fn handle_delete_post(&mut self, id: u64) -> Result<(), Box<dyn Error>> {
        let index = self
            .posts
            .iter()
            .position(|post| post.id == id)
            .ok_or("Post not found")?;
        self.posts.remove(index);

        println!("Deleted post with id {}", id);
        Ok(())
    }
}

struct JsonApp {
    blog_app: Arc<Mutex<BlogApp>>,
}

impl JsonApp {
    fn new() -> Self {
        Self {
            blog_app: Arc::new(Mutex::new(BlogApp::new())),
        }
    }
}

impl Application for JsonApp {
    async fn advance(
        &self,
        env: &impl Environment,
        _metadata: Metadata,
        payload: Vec<u8>,
    ) -> Result<FinishStatus, Box<dyn Error>> {
        let input: Input = serde_json::from_slice(&payload)?;

        let mut app = self.blog_app.lock().await;
        match input {
            Input::AddPost { title, content } => {
                app.handle_add_post(title, content)?;
                env.send_notice(serde_json::to_vec(&format!(
                    "Added post: {}",
                    app.posts.last().expect("Failed to get last post").title
                ))?)
                .await?;
            }
            Input::UpdatePost { id, title, content } => {
                app.handle_update_post(id, title, content)?;
                env.send_notice(serde_json::to_vec(&format!(
                    "Updated post with id: {}",
                    id
                ))?)
                .await?;
            }
            Input::DeletePost { id } => {
                app.handle_delete_post(id)?;
                env.send_notice(serde_json::to_vec(&format!(
                    "Deleted post with id: {}",
                    id
                ))?)
                .await?;
            }
        }
        let report_response = serde_json::to_vec(&app.posts)?;
        env.send_report(report_response).await?;

        Ok(FinishStatus::Accept)
    }

    async fn inspect(
        &self,
        env: &impl Environment,
        _payload: Vec<u8>,
    ) -> Result<FinishStatus, Box<dyn Error>> {
        let app = self.blog_app.lock().await;
        let response = serde_json::to_vec(&app.posts)?;
        env.send_report(response).await?;
        Ok(FinishStatus::Accept)
    }
}

#[async_std::main]
async fn main() {
    let app = JsonApp::new();
    let options = RunOptions::default();
    if let Err(e) = run(app, options).await {
        eprintln!("Error: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_std::sync::Mutex;
    use crabrolls::prelude::*;
    use std::sync::Arc;

    async fn extract_posts(output: &Output) -> Vec<Post> {
        match output {
            Output::Report { payload, .. } => {
                serde_json::from_slice(payload).expect("Failed to deserialize posts from payload")
            }
            _ => vec![],
        }
    }

    async fn extract_string(output: &Output) -> String {
        match output {
            Output::Report { payload, .. } => {
                String::from_utf8(payload.clone()).expect("Failed to convert payload to string")
            }
            _ => String::new(),
        }
    }

    #[async_std::test]
    async fn test_add_post() {
        let app = JsonApp::new();
        let tester = Tester::new(app);

        let add_payload = serde_json::to_vec(&Input::AddPost {
            title: "First Post".into(),
            content: "This is the first post.".into(),
        })
        .expect("Failed to serialize AddPost input");

        let result = tester
            .advance(Address::default(), add_payload.clone())
            .await;

        assert_eq!(result.status, FinishStatus::Accept);
        assert!(result.error.is_none());

        let expected_post = Post {
            id: 1,
            title: "First Post".into(),
            content: "This is the first post.".into(),
        };
        let posts = extract_posts(&result.outputs[1]).await;
        assert_eq!(posts.len(), 1, "Expected one post, found {}", posts.len());
        assert_eq!(posts[0], expected_post);

        let expected_string = serde_json::to_string(&vec![expected_post])
            .expect("Failed to serialize expected post list");
        let result_string = extract_string(&result.outputs[1]).await;
        assert_eq!(
            result_string, expected_string,
            "Expected post list string: {}, but found: {}",
            expected_string, result_string
        );
    }

    #[async_std::test]
    async fn test_update_post() {
        let app = JsonApp::new();
        let tester = Tester::new(app);

        let add_payload = serde_json::to_vec(&Input::AddPost {
            title: "First Post".into(),
            content: "This is the first post.".into(),
        })
        .expect("Failed to serialize AddPost input");

        tester
            .advance(Address::default(), add_payload.clone())
            .await;

        let update_payload = serde_json::to_vec(&Input::UpdatePost {
            id: 1,
            title: Some("Updated First Post".into()),
            content: None,
        })
        .expect("Failed to serialize UpdatePost input");

        let result = tester
            .advance(Address::default(), update_payload.clone())
            .await;

        assert_eq!(result.status, FinishStatus::Accept);
        assert!(result.error.is_none());

        let expected_post = Post {
            id: 1,
            title: "Updated First Post".into(),
            content: "This is the first post.".into(),
        };
        let posts = extract_posts(&result.outputs[1]).await;
        assert_eq!(posts.len(), 1, "Expected one post, found {}", posts.len());
        assert_eq!(posts[0], expected_post);

        let expected_string = serde_json::to_string(&vec![expected_post])
            .expect("Failed to serialize expected post list");
        let result_string = extract_string(&result.outputs[1]).await;
        assert_eq!(
            result_string, expected_string,
            "Expected post list string: {}, but found: {}",
            expected_string, result_string
        );
    }

    #[async_std::test]
    async fn test_delete_post() {
        let app = JsonApp::new();
        let tester = Tester::new(app);

        let add_payload = serde_json::to_vec(&Input::AddPost {
            title: "First Post".into(),
            content: "This is the first post.".into(),
        })
        .expect("Failed to serialize AddPost input");

        tester
            .advance(Address::default(), add_payload.clone())
            .await;

        let delete_payload = serde_json::to_vec(&Input::DeletePost { id: 1 })
            .expect("Failed to serialize DeletePost input");

        let result = tester
            .advance(Address::default(), delete_payload.clone())
            .await;

        assert_eq!(result.status, FinishStatus::Accept);
        assert!(result.error.is_none());

        let posts = extract_posts(&result.outputs[1]).await;
        assert!(posts.is_empty(), "Expected no posts, found {:?}", posts);

        let expected_string = serde_json::to_string(&vec![] as &Vec<Post>)
            .expect("Failed to serialize expected post list");
        let result_string = extract_string(&result.outputs[1]).await;
        assert_eq!(
            result_string, expected_string,
            "Expected post list string: {}, but found: {}",
            expected_string, result_string
        );
    }

    #[async_std::test]
    async fn test_error_handling() {
        let app = JsonApp::new();
        let tester = Tester::new(app);

        let update_payload = serde_json::to_vec(&Input::UpdatePost {
            id: 1,
            title: Some("Updated First Post".into()),
            content: None,
        })
        .expect("Failed to serialize UpdatePost input");

        let result = tester
            .advance(Address::default(), update_payload.clone())
            .await;

        assert_eq!(result.status, FinishStatus::Reject);
        assert!(result.error.is_some());
    }
}
