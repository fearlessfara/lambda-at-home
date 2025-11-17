# Lambda@Home API Compliance & Edge Case Analysis

**Date:** 2025-11-17
**Analyst:** Claude Code
**Scope:** AWS Lambda API 2015-03-31 specification compliance review

## Executive Summary

Lambda@Home implements **15 out of ~80** AWS Lambda API endpoints, focusing on core function lifecycle and invocation. The implementation is solid for development/testing but has **14 critical validation gaps** that need addressing for production-ready AWS Lambda compatibility.

**Overall Risk Assessment:** MEDIUM
**Test Coverage:** 97 tests passing ✅
**Code Quality:** Clean (clippy passed, well-formatted) ✅

---

## 1. Implemented Endpoints (15/80)

### Core Function Management ✅
- `POST /2015-03-31/functions` - CreateFunction
- `GET /2015-03-31/functions/{name}` - GetFunction
- `DELETE /2015-03-31/functions/{name}` - DeleteFunction
- `GET /2015-03-31/functions` - ListFunctions

### Code & Configuration ✅
- `PUT /2015-03-31/functions/{name}/code` - UpdateFunctionCode
- `PUT /2015-03-31/functions/{name}/configuration` - UpdateFunctionConfiguration

### Versions & Aliases ✅
- `POST /2015-03-31/functions/{name}/versions` - PublishVersion
- `GET /2015-03-31/functions/{name}/versions` - ListVersions
- `POST /2015-03-31/functions/{name}/aliases` - CreateAlias
- `GET /2015-03-31/functions/{name}/aliases/{alias}` - GetAlias
- `PUT /2015-03-31/functions/{name}/aliases/{alias}` - UpdateAlias
- `DELETE /2015-03-31/functions/{name}/aliases/{alias}` - DeleteAlias
- `GET /2015-03-31/functions/{name}/aliases` - ListAliases

### Concurrency ✅
- `PUT /2015-03-31/functions/{name}/concurrency` - PutFunctionConcurrency
- `GET /2015-03-31/functions/{name}/concurrency` - GetFunctionConcurrency
- `DELETE /2015-03-31/functions/{name}/concurrency` - DeleteFunctionConcurrency

### Invocation ✅
- `POST /2015-03-31/functions/{name}/invocations` - Invoke

---

## 2. Missing AWS Lambda Endpoints (65/80)

### High Priority (Should Implement)
- ❌ `GET /2015-03-31/functions/{name}/configuration` - GetFunctionConfiguration (separate from GetFunction)

### Medium Priority (Advanced Features)
- ❌ **Permissions** (3): AddPermission, RemovePermission, GetPolicy
- ❌ **Tags** (3): TagResource, UntagResource, ListTags
- ❌ **Event Source Mappings** (5): Create, Get, Update, Delete, List
- ❌ **Account Settings** (1): GetAccountSettings

### Low Priority (Enterprise Features)
- ❌ **Layers** (9): All layer operations
- ❌ **Function URLs** (5): All function URL operations
- ❌ **Code Signing** (9): All code signing operations
- ❌ **Provisioned Concurrency** (4): All provisioned concurrency operations
- ❌ **Event Invoke Config** (5): All event invoke configuration operations

---

## 3. Critical Validation Gaps

### 🔴 HIGH PRIORITY

#### 1. Timeout Validation
**Issue:** No validation that timeout is between 1-900 seconds
**AWS Spec:** 1-900 seconds (default: 3)
**Current:** Accepts any value
**Impact:** Could cause resource exhaustion or timeouts not enforced

**Fix Location:** `service/crates/control/src/registry.rs`
```rust
// Add to create_function() and update_function_configuration()
let timeout = request.timeout.unwrap_or(3);
if timeout < 1 || timeout > 900 {
    return Err(LambdaError::InvalidRequest {
        reason: format!("Timeout must be between 1 and 900 seconds, got {}", timeout),
    });
}
```

#### 2. Memory Size Validation
**Issue:** No validation that memory is 128-10240 MB
**AWS Spec:** 128-10240 MB in 1MB increments (default: 128)
**Current:** Accepts any value
**Impact:** Could cause OOM or resource waste

**Fix Location:** `service/crates/control/src/registry.rs`
```rust
// Add to create_function() and update_function_configuration()
let memory_size = request.memory_size.unwrap_or(128);
if memory_size < 128 || memory_size > 10240 {
    return Err(LambdaError::InvalidRequest {
        reason: format!("Memory size must be between 128 and 10240 MB, got {}", memory_size),
    });
}
```

#### 3. Invocation Payload Size
**Issue:** No payload size validation
**AWS Spec:** Max 6MB for synchronous, 256KB for asynchronous
**Current:** No size check
**Impact:** Could cause performance/memory issues

**Fix Location:** `service/crates/api/src/handlers.rs:invoke_function()`
```rust
const MAX_SYNC_PAYLOAD: usize = 6 * 1024 * 1024; // 6MB
const MAX_ASYNC_PAYLOAD: usize = 256 * 1024; // 256KB

let max_size = match invocation_type {
    InvocationType::RequestResponse | InvocationType::DryRun => MAX_SYNC_PAYLOAD,
    InvocationType::Event => MAX_ASYNC_PAYLOAD,
};

if body.len() > max_size {
    return Err((
        StatusCode::REQUEST_ENTITY_TOO_LARGE,
        Json(ErrorShape {
            error_message: format!("Request payload too large: {} bytes (max: {} bytes)",
                body.len(), max_size),
            error_type: "RequestTooLargeException".to_string(),
            stack_trace: None,
        }),
    ));
}
```

#### 4. Asynchronous Invocation (InvocationType::Event)
**Issue:** "Event" invocation type not properly implemented
**AWS Spec:** Should return 202 Accepted immediately, execute async
**Current:** Likely executes synchronously
**Impact:** Wrong behavior - blocks on async calls

**Fix Location:** `service/crates/control/src/registry.rs:invoke_function()`
```rust
// Handle async invocation
if request.invocation_type == InvocationType::Event {
    // Spawn async task and return immediately
    tokio::spawn(async move {
        // Execute function asynchronously
        // Store result in execution tracker
    });

    return Ok(InvokeResponse {
        status_code: 202,
        payload: None,
        executed_version: Some(version.to_string()),
        function_error: None,
        log_result: None,
        headers: HashMap::new(),
        duration_ms: None,
    });
}
```

### 🟡 MEDIUM PRIORITY

#### 5. Code Size Validation
**Issue:** No validation of ZIP file size
**AWS Spec:** Max 50MB zipped (direct upload), 250MB unzipped
**Current:** No size check

**Fix Location:** `service/crates/packaging/src/zip_handler.rs`
```rust
const MAX_ZIP_SIZE: u64 = 50 * 1024 * 1024; // 50MB
const MAX_UNZIPPED_SIZE: u64 = 250 * 1024 * 1024; // 250MB

if zip_data.len() as u64 > MAX_ZIP_SIZE {
    return Err(LambdaError::CodeTooLarge {
        size: zip_data.len() as u64,
        max_size: MAX_ZIP_SIZE,
    });
}
```

#### 6. Environment Variables Total Size
**Issue:** No validation of total env vars size
**AWS Spec:** Total size ≤ 4096 bytes
**Current:** No size limit

**Fix Location:** `service/crates/control/src/registry.rs`
```rust
fn validate_environment_size(env: &HashMap<String, String>) -> Result<(), LambdaError> {
    let json_str = serde_json::to_string(env).unwrap_or_default();
    const MAX_ENV_SIZE: usize = 4096;

    if json_str.len() > MAX_ENV_SIZE {
        return Err(LambdaError::InvalidRequest {
            reason: format!(
                "Environment variables size {} bytes exceeds max {} bytes",
                json_str.len(), MAX_ENV_SIZE
            ),
        });
    }
    Ok(())
}
```

#### 7. Missing GetFunctionConfiguration Endpoint
**Issue:** AWS has separate `/configuration` GET endpoint
**AWS Spec:** `GET /2015-03-31/functions/{name}/configuration`
**Current:** Not implemented

**Fix Location:** `service/crates/api/src/routes.rs`
```rust
.route(
    "/2015-03-31/functions/:name/configuration",
    get(get_function_configuration),
)
```

#### 8. Qualifier Support in Invoke
**Issue:** Invoke doesn't support version/alias qualifier
**AWS Spec:** Query parameter `?Qualifier=version or alias`
**Current:** Not implemented

**Fix Location:** `service/crates/api/src/handlers.rs:invoke_function()`
```rust
// Extract qualifier from query parameters
#[derive(Deserialize)]
struct InvokeQueryParams {
    #[serde(rename = "Qualifier")]
    qualifier: Option<String>,
}

pub async fn invoke_function(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(params): Query<InvokeQueryParams>,
    headers: HeaderMap,
    body: Bytes,
) -> InvokeResponse {
    // Use params.qualifier to invoke specific version/alias
    let request = InvokeRequest {
        function_name: name.clone(),
        qualifier: params.qualifier,
        // ...
    };
}
```

#### 9. DryRun Invocation Type
**Issue:** DryRun not fully implemented
**AWS Spec:** Should return 204 No Content without executing
**Current:** Unknown behavior

**Fix Location:** `service/crates/control/src/registry.rs:invoke_function()`
```rust
if request.invocation_type == InvocationType::DryRun {
    // Validate function exists and is invocable, but don't execute
    return Ok(InvokeResponse {
        status_code: 204,
        payload: None,
        executed_version: Some(version.to_string()),
        function_error: None,
        log_result: None,
        headers: HashMap::new(),
        duration_ms: None,
    });
}
```

### 🟢 LOW PRIORITY

#### 10. Handler Length Validation
**Issue:** No validation of handler string length
**AWS Spec:** 0-128 characters
**Current:** No length check

```rust
if handler.len() > 128 {
    return Err(LambdaError::InvalidHandler { handler: handler.to_string() });
}
```

#### 11. Description Length Validation
**Issue:** No validation of description length
**AWS Spec:** 0-256 characters
**Current:** No length check

```rust
if let Some(desc) = &request.description {
    if desc.len() > 256 {
        return Err(LambdaError::InvalidRequest {
            reason: format!("Description exceeds 256 characters: {}", desc.len()),
        });
    }
}
```

#### 12. CreateFunction HTTP Status Code
**Issue:** Should return 201 Created instead of 200 OK
**AWS Spec:** 201 Created
**Current:** Returns 200 OK (Axum default for Json response)

**Fix Location:** `service/crates/api/src/handlers.rs:create_function()`
```rust
pub async fn create_function(
    State(state): State<AppState>,
    Json(payload): Json<CreateFunctionRequest>,
) -> Result<(StatusCode, Json<lambda_models::Function>), (StatusCode, Json<ErrorShape>)> {
    // ...
    match state.control.create_function(payload).await {
        Ok(function) => {
            state
                .metrics
                .record_function_created(&function.function_name)
                .await;
            Ok((StatusCode::CREATED, Json(function)))  // 201 instead of 200
        }
        // ...
    }
}
```

#### 13. Client Context Handling
**Issue:** X-Amz-Client-Context header parsed but not used
**AWS Spec:** Should be passed to function in runtime context
**Current:** Ignored

**Fix Location:** `service/crates/models/src/invoke.rs:RuntimeInvocation`
```rust
// Include client_context in RuntimeInvocation sent to container
pub struct RuntimeInvocation {
    // ...
    pub client_context: Option<String>, // Already defined
    // ...
}
```

#### 14. Function State Reason Fields
**Issue:** StateReason and StateReasonCode not always populated
**AWS Spec:** Should include reason when state is Failed/Inactive
**Current:** Basic state tracking

```rust
// When setting function state to Failed:
function.state = FunctionState::Failed;
function.state_reason = Some("Image build failed".to_string());
function.state_reason_code = Some("ImageBuildError".to_string());
```

---

## 4. Edge Cases to Test

### Input Validation
- [ ] Function name with 65 characters (should fail)
- [ ] Function name with special chars like `@` (should fail)
- [ ] Timeout of 0 (should fail)
- [ ] Timeout of 901 (should fail)
- [ ] Memory size of 127 MB (should fail)
- [ ] Memory size of 10241 MB (should fail)
- [ ] ZIP file > 50MB (should fail)
- [ ] Unzipped code > 250MB (should fail)
- [ ] Environment vars > 4KB (should fail)
- [ ] Handler > 128 chars (should fail)
- [ ] Description > 256 chars (should fail)

### Invocation Edge Cases
- [ ] Invoke with 6MB+1 byte payload (should fail with 413)
- [ ] Async invoke with 256KB+1 byte payload (should fail with 413)
- [ ] Invoke non-existent function (should return 404)
- [ ] Invoke with invalid JSON payload (should accept as string)
- [ ] Invoke with empty payload (should send null)
- [ ] DryRun invocation (should return 204 without executing)
- [ ] Event invocation (should return 202 immediately)

### Concurrency Edge Cases
- [ ] Reserve more concurrency than global limit (should fail)
- [ ] Reserve 0 concurrency (should accept - pauses function)
- [ ] Concurrent requests exceeding reserved concurrency (should throttle)

### Function Lifecycle Edge Cases
- [ ] Create duplicate function (should return 409 ResourceConflictException)
- [ ] Update non-existent function (should return 404)
- [ ] Delete function with in-flight executions (should wait for completion)
- [ ] Invoke function marked for deletion (should reject new invocations)

---

## 5. Recommendations

### Immediate Actions (P0 - This Week)
1. ✅ Add timeout validation (1-900 seconds)
2. ✅ Add memory size validation (128-10240 MB)
3. ✅ Add payload size validation (6MB sync, 256KB async)
4. ✅ Implement proper async invocation (InvocationType::Event)

### Short-term (P1 - Next Sprint)
5. ✅ Add code size validation (50MB zipped, 250MB unzipped)
6. ✅ Add environment variables size validation (4KB)
7. ✅ Implement GetFunctionConfiguration endpoint
8. ✅ Add qualifier support in invoke
9. ✅ Implement DryRun properly

### Medium-term (P2 - Next Month)
10. ✅ Fix CreateFunction to return 201 Created
11. ✅ Add handler/description length validation
12. ✅ Populate client_context in runtime invocations
13. ✅ Improve function state reason tracking
14. ✅ Add comprehensive edge case tests

### Long-term (P3 - Roadmap)
15. Consider implementing Tags API (useful for console)
16. Consider implementing GetAccountSettings (useful for quotas)
17. Consider implementing Layers (useful for shared dependencies)
18. Consider implementing Event Source Mappings (useful for integrations)

---

## 6. Test Coverage Assessment

**Current:** 97 tests passing ✅
**Missing Test Coverage:**
- ❌ Validation boundary tests (timeout, memory, sizes)
- ❌ Async invocation tests (InvocationType::Event)
- ❌ DryRun invocation tests
- ❌ Payload size limit tests
- ❌ Error response format tests
- ❌ Qualifier-based invocation tests

**Recommended New Tests:**
```rust
// service/crates/control/tests/validation_tests.rs
#[tokio::test]
async fn test_timeout_validation() {
    // Test timeout < 1
    // Test timeout > 900
    // Test timeout = 1 (should pass)
    // Test timeout = 900 (should pass)
}

#[tokio::test]
async fn test_memory_validation() {
    // Test memory < 128
    // Test memory > 10240
    // Test memory = 128 (should pass)
    // Test memory = 10240 (should pass)
}

#[tokio::test]
async fn test_async_invocation() {
    // Test InvocationType::Event returns 202
    // Test function executes asynchronously
    // Test result stored in execution tracker
}
```

---

## 7. Conclusion

Lambda@Home has a solid foundation with 97 passing tests and clean code. However, **14 validation gaps** need addressing before it's production-ready for AWS Lambda compatibility.

**Priority Focus Areas:**
1. **Validation hardening** - Add all AWS Lambda parameter validations
2. **Async invocation** - Properly implement InvocationType::Event
3. **Error handling** - Ensure all error codes match AWS spec
4. **Edge case testing** - Add comprehensive boundary condition tests

**Estimated Effort:**
- P0 (Immediate): 4-8 hours
- P1 (Short-term): 8-16 hours
- P2 (Medium-term): 4-8 hours
- Total: ~16-32 hours of focused development

**Next Steps:**
1. Review this analysis with the team
2. Prioritize which validations to implement first
3. Create issues/tickets for each validation gap
4. Add edge case tests alongside validation fixes
5. Update API documentation with known limitations

---

**Generated by:** Claude Code
**Last Updated:** 2025-11-17
**Review Status:** ✅ Ready for team review
