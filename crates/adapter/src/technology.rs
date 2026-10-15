//! Reads a text for the technology it names.
//!
//! A specification states behaviour by the role a thing plays, and the
//! design holds the product, protocol, library, framework, or host that plays
//! it. [`TECHNOLOGY`] is the fixed lexicon of such names, [`package_terms`]
//! reads the names a run adds to it from the packages a source imports, and
//! [`names_technology`] is the one rule every gate over a specification's
//! words applies.

/// The fixed names of vendors, products, protocols, frameworks, and hosts.
///
/// Every entry is lowercase letters and digits with no separator, so a text
/// spelling one with a space, a hyphen, or a case change still names it:
/// `Key Vault`, `key-vault`, and `KeyVault` all spell `keyvault`. A name that
/// is also a plain word of a domain (`express`, `spring`, `rabbit`, `vault`)
/// is left out, and its product spelled in full (`expressjs`, `springboot`,
/// `rabbitmq`, `hashicorp`), so the domain's own words are never findings.
pub const TECHNOLOGY: &[&str] = &[
    // clouds and their services
    "aws",
    "amazon",
    "ec2",
    "s3",
    "sqs",
    "sns",
    "kinesis",
    "dynamodb",
    "cloudwatch",
    "eventbridge",
    "secretsmanager",
    "cloudfront",
    "route53",
    "fargate",
    "eks",
    "cognito",
    "azure",
    "keyvault",
    "eventhub",
    "eventhubs",
    "eventgrid",
    "servicebus",
    "cosmosdb",
    "aks",
    "gcp",
    "google",
    "pubsub",
    "bigquery",
    "firestore",
    "gke",
    "cloudflare",
    "fastly",
    "heroku",
    "vercel",
    "netlify",
    "digitalocean",
    "firebase",
    "supabase",
    "hashicorp",
    // brokers and streams
    "kafka",
    "kafkajs",
    "confluent",
    "confluentinc",
    "rabbitmq",
    "amqp",
    "mqtt",
    "nats",
    "pulsar",
    "zeromq",
    "activemq",
    "mosquitto",
    "bullmq",
    "sidekiq",
    "celery",
    "dramatiq",
    "temporalio",
    // stores and caches
    "redis",
    "ioredis",
    "memcached",
    "postgres",
    "postgresql",
    "mysql",
    "mariadb",
    "sqlite",
    "mssql",
    "sqlserver",
    "mongo",
    "mongodb",
    "mongoose",
    "couchdb",
    "couchbase",
    "cassandra",
    "elasticsearch",
    "opensearch",
    "neo4j",
    "influxdb",
    "clickhouse",
    "duckdb",
    "cockroachdb",
    "redshift",
    "databricks",
    "prisma",
    "sequelize",
    "typeorm",
    "knex",
    "sqlalchemy",
    "alembic",
    // frameworks and runtimes
    "expressjs",
    "fastify",
    "nestjs",
    "koa",
    "hapi",
    "hono",
    "nextjs",
    "django",
    "flask",
    "fastapi",
    "starlette",
    "uvicorn",
    "gunicorn",
    "tornado",
    "aiohttp",
    "springboot",
    "nodejs",
    "typescript",
    "javascript",
    "python",
    "java",
    "kotlin",
    "golang",
    "dotnet",
    "csharp",
    // clients and libraries
    "axios",
    "undici",
    "httpx",
    "boto3",
    "winston",
    "pino",
    "bunyan",
    "loguru",
    "structlog",
    "dotenv",
    "pydantic",
    "lodash",
    "dayjs",
    "luxon",
    "nodemailer",
    "socketio",
    "puppeteer",
    "jsonwebtoken",
    "pyjwt",
    "bcrypt",
    "argon2",
    // protocols and formats
    "grpc",
    "graphql",
    "protobuf",
    "avro",
    "websocket",
    "websockets",
    "smtp",
    "imap",
    "ldap",
    "saml",
    "openapi",
    "swagger",
    // identity, payments, and messaging services
    "auth0",
    "okta",
    "keycloak",
    "stripe",
    "paypal",
    "braintree",
    "twilio",
    "sendgrid",
    "mailgun",
    "slack",
    "discord",
    "salesforce",
    "hubspot",
    "zendesk",
    // observability
    "datadog",
    "newrelic",
    "splunk",
    "sentry",
    "grafana",
    "prometheus",
    "jaeger",
    "zipkin",
    "opentelemetry",
    "otel",
    "pagerduty",
    "opsgenie",
    "mixpanel",
    "launchdarkly",
    // platforms and tooling
    "kubernetes",
    "k8s",
    "docker",
    "terraform",
    "ansible",
    "jenkins",
    "github",
    "gitlab",
    "bitbucket",
    "nginx",
    "istio",
    "linkerd",
    "etcd",
    "zookeeper",
    "hadoop",
    "flink",
    "airflow",
    "dagster",
    "webpack",
    "eslint",
    "vitest",
    "jest",
    "pytest",
    "selenium",
    // AI services
    "openai",
    "anthropic",
    "huggingface",
    "langchain",
    "pinecone",
    "weaviate",
    "qdrant",
    "milvus",
];

// The most words a lexicon entry or a term may be spelled across: `Key
// Vault`, `Cosmos DB`, `Google Cloud Pub Sub`.
const RUN: usize = 3;

// What marks a lone word of a package name as coined rather than plain:
// `boto3`, `kafkajs`, `mongodb`, `bullmq`, `sqlalchemy`, `pymongo`.
const COINED_LEADS: &[&str] = &["py"];
const COINED_TAILS: &[&str] = &["js", "db", "mq"];
const COINED_PARTS: &[&str] = &["sql"];

/// Returns the technology `text` names, from [`TECHNOLOGY`] and `terms`.
///
/// `text` is read as words: runs of letters and digits, split again at a
/// camel-case boundary and lowercased, so `useConfluentKafkaConfig`,
/// `KEYVAULT_HOST`, and `key-vault.refuse-empty` read as the words they
/// join. A word, or the words of up to three in a row joined, that equals an
/// entry of the lexicon or one of `terms` names that entry; each entry is
/// returned once, in the order the text reaches it, spelled as the lexicon
/// spells it. Nothing is returned for a text naming none.
///
/// `terms` are the names a run derives from the packages a source imports,
/// through [`package_terms`], each spelled as an entry is.
///
/// # Examples
///
/// ```
/// use emery_adapter::names_technology;
///
/// let terms = vec!["keyvaultsecrets".to_owned()];
/// assert_eq!(
///     names_technology(
///         "Credentials are read from Key Vault before the Kafka clients start.",
///         &[]
///     ),
///     ["keyvault", "kafka"]
/// );
/// assert_eq!(names_technology("useConfluentKafkaConfig", &[]), ["confluent", "kafka"]);
/// assert_eq!(names_technology("KEYVAULT_HOST", &[]), ["keyvault"]);
/// assert_eq!(names_technology("key-vault.refuse-empty", &[]), ["keyvault"]);
/// assert_eq!(names_technology("the keyvault-secrets client", &terms), ["keyvaultsecrets"]);
/// assert!(names_technology("POST /orders creates an order.", &[]).is_empty());
/// assert!(
///     names_technology("Each converted event is published to the outbound stream.", &[])
///         .is_empty()
/// );
/// ```
#[must_use]
pub fn names_technology(text: &str, terms: &[String]) -> Vec<String> {
    let words = words(text);
    let mut named: Vec<String> = Vec::new();
    let mut start = 0;
    while start < words.len() {
        // the longest run from here that spells an entry
        let mut joined = String::new();
        let mut hit: Option<(usize, &str)> = None;
        for (length, word) in (1..).zip(words.iter().skip(start).take(RUN)) {
            if length > 1 && word.len() < 2 {
                break;
            }
            joined.push_str(word);
            if let Some(entry) = entry(&joined, terms) {
                hit = Some((length, entry));
            }
        }

        match hit {
            Some((length, entry)) => {
                if !named.iter().any(|named| named == entry) {
                    named.push(entry.to_owned());
                }
                start += length;
            }
            None => start += 1,
        }
    }
    named
}

/// Returns the technology terms a package `specifier` adds to the lexicon.
///
/// A term is spelled as a lexicon entry is, lowercase letters and digits
/// with no separator, so [`names_technology`] reads it the same way. The
/// specifier yields:
///
/// - the scope of a scoped package, which always names a vendor or a
///   product: `@azure`, `@aws-sdk`, `@nestjs`
/// - a compound name, joined: `keyvault-secrets`, `confluent_kafka`,
///   `socket.io`, which a text names only by spelling both parts
/// - a lone name that is coined rather than plain: one holding a digit, led
///   by `py`, ending in `js`, `db`, or `mq`, or holding `sql`
///
/// A lone plain word (`requests`, `events`, `express`) yields nothing, since
/// it would make the domain's own words findings; the lexicon names the
/// products among them. A path beneath the package (`lodash/fp`) is not
/// read.
///
/// # Examples
///
/// ```
/// use emery_adapter::package_terms;
///
/// assert_eq!(package_terms("@azure/keyvault-secrets"), ["azure", "keyvaultsecrets"]);
/// assert_eq!(package_terms("@nestjs/common"), ["nestjs"]);
/// assert_eq!(package_terms("kafkajs"), ["kafkajs"]);
/// assert_eq!(package_terms("confluent_kafka"), ["confluentkafka"]);
/// assert_eq!(package_terms("boto3"), ["boto3"]);
/// assert!(package_terms("requests").is_empty());
/// assert!(package_terms("express").is_empty());
/// ```
#[must_use]
pub fn package_terms(specifier: &str) -> Vec<String> {
    let mut terms = Vec::new();
    let (scope, name) = specifier.strip_prefix('@').map_or((None, specifier), |scoped| {
        let (scope, name) = scoped.split_once('/').unwrap_or((scoped, ""));
        (Some(scope), name)
    });

    // the scope is a vendor's or a product's whatever word it is
    if let Some(scope) = scope {
        let parts = parts(scope);
        if !parts.is_empty() {
            terms.push(parts.concat());
        }
    }

    // the name counts when a text could spell it only on purpose
    let name = name.split('/').next().unwrap_or(name);
    let parts = parts(name);
    let coined = match parts.as_slice() {
        [] => false,
        [one] => {
            one.bytes().any(|byte| byte.is_ascii_digit())
                || COINED_LEADS.iter().any(|lead| one.starts_with(lead) && one.len() > lead.len())
                || COINED_TAILS.iter().any(|tail| one.ends_with(tail) && one.len() > tail.len())
                || COINED_PARTS.iter().any(|part| one.contains(part))
        }
        _ => true,
    };
    if coined {
        let term = parts.concat();
        if !terms.contains(&term) {
            terms.push(term);
        }
    }

    terms
}

// The parts of a package name: runs of letters and digits between its
// separators, lowercased, with no camel-case split since a package name
// has none.
fn parts(name: &str) -> Vec<String> {
    name.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

// The words of a text: runs of letters and digits, split again where a
// lowercase letter or a digit meets an uppercase one, or an uppercase run
// meets its last letter's lowercase successor (`IORedis` is `io`, `redis`).
fn words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let chars: Vec<char> = text.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if !c.is_ascii_alphanumeric() {
            if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
            continue;
        }
        if c.is_ascii_uppercase() && i > 0 && !word.is_empty() {
            let prev = chars[i - 1];
            let next = chars.get(i + 1).copied();
            let hump = prev.is_ascii_lowercase()
                || prev.is_ascii_digit()
                || (prev.is_ascii_uppercase() && next.is_some_and(|n| n.is_ascii_lowercase()));
            if hump {
                words.push(std::mem::take(&mut word));
            }
        }
        word.push(c.to_ascii_lowercase());
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}

fn entry<'t>(joined: &str, terms: &'t [String]) -> Option<&'t str> {
    TECHNOLOGY
        .iter()
        .copied()
        .find(|entry| *entry == joined)
        .or_else(|| terms.iter().map(String::as_str).find(|term| *term == joined))
}
