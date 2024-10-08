use async_std::sync::RwLock;
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
		println!("Added post {:?}", self.posts.last().expect("Failed to get last post"));
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
	blog_app: Arc<RwLock<BlogApp>>,
}

impl JsonApp {
	fn new() -> Self {
		Self {
			blog_app: Arc::new(RwLock::new(BlogApp::new())),
		}
	}
}

impl Application for JsonApp {
	async fn advance(
		&self,
		env: &impl Environment,
		_metadata: Metadata,
		payload: &[u8],
		_deposit: Option<Deposit>,
	) -> Result<FinishStatus, Box<dyn Error>> {
		let input: Input = serde_json::from_slice(payload)?;

		let mut app = self.blog_app.write().await;
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
				env.send_notice(serde_json::to_vec(&format!("Updated post with id: {}", id))?)
					.await?;
			}
			Input::DeletePost { id } => {
				app.handle_delete_post(id)?;
				env.send_notice(serde_json::to_vec(&format!("Deleted post with id: {}", id))?)
					.await?;
			}
		}
		let report_response = serde_json::to_vec(&app.posts)?;
		env.send_report(report_response).await?;

		Ok(FinishStatus::Accept)
	}

	async fn inspect(&self, env: &impl Environment, _payload: &[u8]) -> Result<FinishStatus, Box<dyn Error>> {
		let app = self.blog_app.read().await;
		let response = serde_json::to_vec(&app.posts)?;
		env.send_report(response).await?;
		Ok(FinishStatus::Accept)
	}
}

#[async_std::main]
async fn main() {
	let app = JsonApp::new();
	let options = RunOptions::default();
	if let Err(e) = Supervisor::run(app, options).await {
		eprintln!("Error: {}", e);
	}
}
#[cfg(test)]
mod tests {
	use super::*;
	use ethabi::Address;
	use serde_json::json;

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
		let tester = Tester::new(app, MockupOptions::default());

		let add_payload = json!({
			"kind": "AddPost",
			"payload": {
				"title": "First Post",
				"content": "This is the first post."
			}
		})
		.to_string();

		let result = tester.advance(Address::default(), add_payload.into_bytes()).await;

		assert!(result.is_accepted(), "Expected Accept status");
		assert!(result.get_error().is_none(), "Expected no error");

		let expected_post = Post {
			id: 1,
			title: "First Post".into(),
			content: "This is the first post.".into(),
		};
		let posts = extract_posts(&result.get_outputs()[1]).await;
		assert_eq!(posts.len(), 1, "Expected one post, found {}", posts.len());
		assert_eq!(posts[0], expected_post);

		let expected_string =
			serde_json::to_string(&vec![expected_post]).expect("Failed to serialize expected post list");
		let result_string = extract_string(&result.get_outputs()[1]).await;
		assert_eq!(
			result_string, expected_string,
			"Expected post list string: {}, but found: {}",
			expected_string, result_string
		);
	}

	#[async_std::test]
	async fn test_update_post() {
		let app = JsonApp::new();
		let tester = Tester::new(app, MockupOptions::default());

		let add_payload = json!({
			"kind": "AddPost",
			"payload": {
				"title": "First Post",
				"content": "This is the first post."
			}
		})
		.to_string();

		tester.advance(Address::default(), add_payload.into_bytes()).await;

		let update_payload = json!({
			"kind": "UpdatePost",
			"payload": {
				"id": 1,
				"title": "Updated First Post",
				"content": null
			}
		})
		.to_string();

		let result = tester.advance(Address::default(), update_payload.into_bytes()).await;

		assert!(result.is_accepted(), "Expected Accept status");
		assert!(result.get_error().is_none(), "Expected no error");

		let expected_post = Post {
			id: 1,
			title: "Updated First Post".into(),
			content: "This is the first post.".into(),
		};
		let posts = extract_posts(&result.get_outputs()[1]).await;
		assert_eq!(posts.len(), 1, "Expected one post, found {}", posts.len());
		assert_eq!(posts[0], expected_post);

		let expected_string =
			serde_json::to_string(&vec![expected_post]).expect("Failed to serialize expected post list");
		let result_string = extract_string(&result.get_outputs()[1]).await;
		assert_eq!(
			result_string, expected_string,
			"Expected post list string: {}, but found: {}",
			expected_string, result_string
		);
	}

	#[async_std::test]
	async fn test_delete_post() {
		let app = JsonApp::new();
		let tester = Tester::new(app, MockupOptions::default());

		let add_payload = json!({
			"kind": "AddPost",
			"payload": {
				"title": "First Post",
				"content": "This is the first post."
			}
		})
		.to_string();

		tester.advance(Address::default(), add_payload.into_bytes()).await;

		let delete_payload = json!({
			"kind": "DeletePost",
			"payload": {
				"id": 1
			}
		})
		.to_string();

		let result = tester.advance(Address::default(), delete_payload.into_bytes()).await;

		assert!(result.is_accepted(), "Expected Accept status");
		assert!(result.get_error().is_none(), "Expected no error");

		let posts = extract_posts(&result.get_outputs()[1]).await;
		assert!(posts.is_empty(), "Expected no posts, found {:?}", posts);

		let expected_string =
			serde_json::to_string(&vec![] as &Vec<Post>).expect("Failed to serialize expected post list");
		let result_string = extract_string(&result.get_outputs()[1]).await;
		assert_eq!(
			result_string, expected_string,
			"Expected post list string: {}, but found: {}",
			expected_string, result_string
		);
	}

	#[async_std::test]
	async fn test_error_handling() {
		let app = JsonApp::new();
		let tester = Tester::new(app, MockupOptions::default());

		let update_payload = json!({
			"kind": "UpdatePost",
			"payload": {
				"id": 1,
				"title": "Updated First Post",
				"content": null
			}
		})
		.to_string();

		let result = tester.advance(Address::default(), update_payload.into_bytes()).await;

		assert!(result.is_rejected(), "Expected Reject status");
		assert!(result.get_error().is_some(), "Expected an error");
	}
}
