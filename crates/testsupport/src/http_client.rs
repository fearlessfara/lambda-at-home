use anyhow::Result;
use lambda_models::{
    CreateFunctionRequest, UpdateFunctionCodeRequest, UpdateFunctionConfigurationRequest,
    PublishVersionRequest, CreateAliasRequest, UpdateAliasRequest, ConcurrencyConfig,
    InvokeResponse, Function, Version, Alias,
};
use reqwest::Client;
use serde_json::Value;

pub struct LambdaClient {
    client: Client,
    base_url: String,
}

impl LambdaClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    pub async fn create_function(&self, request: CreateFunctionRequest) -> Result<Function> {
        let response = self
            .client
            .post(&format!("{}/2015-03-31/functions", self.base_url))
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Create function failed: {}", error_text);
        }
        
        Ok(response.json().await?)
    }

    pub async fn get_function(&self, name: &str) -> Result<Function> {
        let response = self
            .client
            .get(&format!("{}/2015-03-31/functions/{}", self.base_url, name))
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Get function failed: {}", error_text);
        }
        
        Ok(response.json().await?)
    }

    pub async fn delete_function(&self, name: &str) -> Result<()> {
        let response = self
            .client
            .delete(&format!("{}/2015-03-31/functions/{}", self.base_url, name))
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Delete function failed: {}", error_text);
        }
        
        Ok(())
    }

    pub async fn update_function_code(&self, name: &str, request: UpdateFunctionCodeRequest) -> Result<Function> {
        let response = self
            .client
            .put(&format!("{}/2015-03-31/functions/{}/code", self.base_url, name))
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Update function code failed: {}", error_text);
        }
        
        Ok(response.json().await?)
    }

    pub async fn update_function_configuration(&self, name: &str, request: UpdateFunctionConfigurationRequest) -> Result<Function> {
        let response = self
            .client
            .put(&format!("{}/2015-03-31/functions/{}/configuration", self.base_url, name))
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Update function configuration failed: {}", error_text);
        }
        
        Ok(response.json().await?)
    }

    pub async fn publish_version(&self, name: &str, request: PublishVersionRequest) -> Result<Version> {
        let response = self
            .client
            .post(&format!("{}/2015-03-31/functions/{}/versions", self.base_url, name))
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Publish version failed: {}", error_text);
        }
        
        Ok(response.json().await?)
    }

    pub async fn create_alias(&self, name: &str, request: CreateAliasRequest) -> Result<Alias> {
        let response = self
            .client
            .post(&format!("{}/2015-03-31/functions/{}/aliases", self.base_url, name))
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Create alias failed: {}", error_text);
        }
        
        Ok(response.json().await?)
    }

    pub async fn update_alias(&self, name: &str, alias: &str, request: UpdateAliasRequest) -> Result<Alias> {
        let response = self
            .client
            .put(&format!("{}/2015-03-31/functions/{}/aliases/{}", self.base_url, name, alias))
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Update alias failed: {}", error_text);
        }
        
        Ok(response.json().await?)
    }

    pub async fn delete_alias(&self, name: &str, alias: &str) -> Result<()> {
        let response = self
            .client
            .delete(&format!("{}/2015-03-31/functions/{}/aliases/{}", self.base_url, name, alias))
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Delete alias failed: {}", error_text);
        }
        
        Ok(())
    }

    pub async fn put_concurrency(&self, name: &str, config: ConcurrencyConfig) -> Result<()> {
        let response = self
            .client
            .put(&format!("{}/2015-03-31/functions/{}/concurrency", self.base_url, name))
            .json(&config)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Put concurrency failed: {}", error_text);
        }
        
        Ok(())
    }

    pub async fn invoke(&self, name: &str, payload: Value, invocation_type: Option<(&str, &str)>) -> Result<InvokeResponse> {
        let mut request = self
            .client
            .post(&format!("{}/2015-03-31/functions/{}/invocations", self.base_url, name))
            .json(&payload);
        
        if let Some((inv_type, log_type)) = invocation_type {
            request = request
                .header("X-Amz-Invocation-Type", inv_type)
                .header("X-Amz-Log-Type", log_type);
        }
        
        let response = request.send().await?;
        
        let status_code = response.status().as_u16();
        let executed_version = response
            .headers()
            .get("X-Amz-Executed-Version")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());
        
        let function_error = response
            .headers()
            .get("X-Amz-Function-Error")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());
        
        let log_result = response
            .headers()
            .get("X-Amz-Log-Result")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());
        
        let payload: Value = response.json().await?;
        
        Ok(InvokeResponse {
            status_code,
            payload: Some(payload),
            executed_version,
            function_error: function_error.map(|s| match s.as_str() {
                "Handled" => lambda_models::FunctionError::Handled,
                "Unhandled" => lambda_models::FunctionError::Unhandled,
                _ => lambda_models::FunctionError::Unhandled,
            }),
            log_result,
            headers: std::collections::HashMap::new(),
        })
    }
}

// Convenience functions for tests
pub async fn create_function(daemon: &TestDaemon, request: Value) -> Result<Function> {
    let client = LambdaClient::new(daemon.user_api_url.clone());
    let request: CreateFunctionRequest = serde_json::from_value(request)?;
    client.create_function(request).await
}

pub async fn invoke(daemon: &TestDaemon, name: &str, payload: Value, invocation_type: Option<(&str, &str)>) -> Result<InvokeResponse> {
    let client = LambdaClient::new(daemon.user_api_url.clone());
    client.invoke(name, payload, invocation_type).await
}

pub async fn get_function(daemon: &TestDaemon, name: &str) -> Result<Function> {
    let client = LambdaClient::new(daemon.user_api_url.clone());
    client.get_function(name).await
}

pub async fn delete_function(daemon: &TestDaemon, name: &str) -> Result<()> {
    let client = LambdaClient::new(daemon.user_api_url.clone());
    client.delete_function(name).await
}

pub async fn put_concurrency(daemon: &TestDaemon, name: &str, config: ConcurrencyConfig) -> Result<()> {
    let client = LambdaClient::new(daemon.user_api_url.clone());
    client.put_concurrency(name, config).await
}

use crate::daemon::TestDaemon;
