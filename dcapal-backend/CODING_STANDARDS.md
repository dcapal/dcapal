# Backend coding standards

## 1. REST endpoint to service and persistence

Use this structure for every new or modified backend use case:

```text
Axum REST handler
    -> Command::try_new
    -> application service
    -> outbound trait
    -> database, cache, or HTTP adapter
```

The handler adapts HTTP to the application. The command validates and resolves
the request context. The service owns the business decision and workflow. An
outbound adapter performs the database, cache, or external HTTP operation.

This standard applies to new and modified code. Existing code may still be on
an older path; this document does not require a broad migration when adding
these files.

### REST handlers adapt HTTP

Keep an Axum handler as a short orchestration boundary. It should:

1. Extract path, query, header, authentication, and JSON values with Axum.
2. Obtain the relevant service and validation ports from application state.
3. Call the command's fallible `try_new` constructor.
4. Pass the validated command to the service method for the use case.
5. Convert the application result into the REST response, such as JSON and an
   HTTP status, and let the application error convert at the same boundary.

Axum extractors own formal validation: parsing a UUID, decoding JSON, reading
an integer, or checking that a required field is present. Commands own
semantic validation: checking ownership, resource existence, cross-field
rules, authorization facts, and other rules that require application context.

The handler should not make a business decision, perform a resource lookup,
call a database or provider directly for a use-case operation, or assemble a
workflow from several persistence calls. Those decisions belong below the
REST boundary. A handler's body should read as request adaptation, command
construction, service invocation, and response adaptation.

Keep REST request and response types at the inbound boundary. Services and
outbound ports use application or domain types instead of `axum::Json`,
`axum::response::Response`, HTTP extractors, or other transport types.

### Commands validate and resolve context

Create one command for each meaningful application action. Use a fallible
constructor named `try_new`:

```rust
pub struct UpdatePortfolioCommand {
    pub user_id: Uuid,
    pub portfolio_id: Uuid,
    pub current: Portfolio,
    pub update: PortfolioUpdate,
}

impl UpdatePortfolioCommand {
    pub async fn try_new(
        user_id: Uuid,
        portfolio_id: Uuid,
        update: PortfolioUpdate,
        portfolios: &dyn PortfolioRepository,
    ) -> Result<Self> {
        let current = portfolios
            .find_owned(user_id, portfolio_id)
            .await?
            .ok_or_else(|| DcaError::BadRequest("Portfolio cannot be updated".into()))?;

        Ok(Self {
            user_id,
            portfolio_id,
            current,
            update,
        })
    }
}
```

The example shows the required shape; use the domain types and repository
operations of the specific use case. `try_new` should:

- accept the formally parsed input and the outbound ports needed to validate
  or resolve it;
- normalize values into the types the service needs;
- resolve identities, ownership, existence, and other semantic facts;
- return `Result<Self>` so the handler receives either a complete command or
  an application validation error; and
- perform reads needed for validation while leaving writes and action
  execution to the service.

After construction, the command is the service's validated input. The service
can use its resolved context directly instead of repeating the same semantic
checks.

If a command has validation failures that need distinct semantics, define a
command-specific error enum with `thiserror`, such as
`UpdatePortfolioCommandError`, using
`#[derive(Debug, thiserror::Error)]`. Otherwise, return the service or
application error type already appropriate for the use case. Keep command
errors transport-independent and convert them at the REST boundary.

### Services own business logic

Represent a service as a Rust struct constructed with its execution
dependencies. Store outbound dependencies behind traits, usually as
`Arc<dyn Trait>` when the service is shared through `AppContext`.

Each service defines its own error type, such as `PortfolioServiceError`, with
`#[derive(Debug, thiserror::Error)]`. Its variants describe the business and
dependency failures that belong to that service. Wrap underlying causes with
`#[source]` or `#[from]` so the error chain remains available to logging and
boundary conversion. Keep service-specific variants out of the global error
enum unless they represent a concern shared by multiple application
boundaries.

```rust
#[derive(Debug, thiserror::Error)]
pub enum PortfolioServiceError {
    #[error("portfolio cannot be updated")]
    CannotUpdate,
    #[error("portfolio persistence failed")]
    Persistence(#[from] DcaError),
}

pub struct PortfolioService {
    portfolios: Arc<dyn PortfolioRepository>,
}

impl PortfolioService {
    pub fn new(portfolios: Arc<dyn PortfolioRepository>) -> Self {
        Self { portfolios }
    }

    pub async fn update_portfolio(
        &self,
        command: UpdatePortfolioCommand,
    ) -> std::result::Result<Portfolio, PortfolioServiceError> {
        let updated = command.current.apply(command.update)?;

        self.portfolios
            .update(command.user_id, command.portfolio_id, updated)
            .await
    }
}
```

The service method owns the use-case workflow: applying business rules,
coordinating domain operations, choosing the order of actions, and returning
an application or domain result. It returns a value such as a domain entity,
application response, or `Result<T, ServiceError>`. It does not return an
Axum response and does not know how the result will be represented over HTTP.

The service owns execution dependencies. A command receives only the ports it
needs to validate and resolve its context; the service retains the ports used
to perform the action. Construct services once during application setup and
share them through the existing application context.

### Outbound ports own external operations

Define every external dependency used by a command or service as a trait:

```rust
#[async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait PortfolioRepository: Send + Sync {
    async fn find_owned(
        &self,
        user_id: Uuid,
        portfolio_id: Uuid,
    ) -> Result<Option<Portfolio>>;

    async fn update(
        &self,
        user_id: Uuid,
        portfolio_id: Uuid,
        portfolio: Portfolio,
    ) -> Result<Portfolio>;
}
```

Place the trait at the outbound port boundary, such as
`crates/backend/src/ports/outbound/repository` or
`crates/backend/src/ports/outbound/adapter`. Put SQLx, Redis, and concrete
HTTP implementations below the appropriate repository or adapter module.
The service depends on the trait; application startup supplies the production
implementation.

Keep outbound methods expressed in application or domain terms. Persistence
adapters own SQL, row mapping, connection handling, transactions, and storage
invariants. HTTP adapters own request construction, transport details, and
provider response decoding. The service remains responsible for the business
meaning of the operation.

Use `mockall` in test-only dependencies for traits that commands or services
consume. Unit tests can then set expectations on `MockPortfolioRepository` or
the corresponding generated mock without starting PostgreSQL, Redis, or a
real provider. Production code continues to depend on the trait rather than
on the mock.

### Errors cross the boundary deliberately

Keep the error flow explicit:

```text
Command::try_new -> CommandError or application error
Service method   -> ServiceError
REST handler     -> HTTP response conversion
```

Let command validation and service failures remain application errors until
the REST boundary. The handler or the existing error response implementation
converts them into the appropriate HTTP status and body. Preserve the
difference between a client validation error, a missing resource, an external
service failure, and an internal persistence failure; do not collapse every
service error into `400 Bad Request`.
