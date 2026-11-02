#![warn(missing_docs, clippy::missing_errors_doc)]

//! Reads a text for the technology it names.
//!
//! A specification states behaviour by the role a thing plays. The design
//! names the product, protocol, library, framework, or host that implements
//! the behaviour. [`Lexicon`] holds the names specification text is held
//! against, and [`words`] splits identifiers and prose into the lowercase
//! word runs [`Lexicon::names`] matches.
//!
//! # Examples
//!
//! ```
//! use emery_lexicon::Lexicon;
//!
//! let lexicon = Lexicon::from_packages(["@azure/keyvault-secrets"]);
//! assert_eq!(
//!     lexicon.names("Secrets are read from Key Vault before the Kafka clients start."),
//!     ["keyvault", "kafka"]
//! );
//! assert!(lexicon.names("POST /orders creates an order.").is_empty());
//! ```

// The most words a lexicon entry or a package term may span: `Key Vault`,
// `Cosmos DB`, and `Key Vault Secrets` from `@azure/keyvault-secrets`.
const RUN: usize = 3;

/// The technology names a text is held against.
///
/// Every lexicon carries the fixed names of vendors, products, protocols,
/// frameworks, and hosts. A run adds the terms of the packages its source
/// imports through [`Lexicon::from_packages`]. A name that is also a plain
/// word of a domain (`express`, `vault`) is left out of the fixed names, so a
/// text in the domain's own words names nothing.
///
/// [`Lexicon::default`] holds the fixed names alone.
///
/// # Examples
///
/// ```
/// use emery_lexicon::Lexicon;
///
/// let fixed = Lexicon::default();
/// assert_eq!(
///     fixed.names("Credentials are read from Key Vault before the Kafka clients start."),
///     ["keyvault", "kafka"]
/// );
/// assert!(fixed.names("POST /orders creates an order.").is_empty());
///
/// let run = Lexicon::from_packages(["@azure/keyvault-secrets"]);
/// assert_eq!(run.names("the keyvault-secrets client"), ["keyvaultsecrets"]);
/// ```
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Lexicon {
    terms: Vec<String>,
}

impl Lexicon {
    /// Returns the lexicon with `terms` beside the fixed names.
    ///
    /// Each term is lowercase ASCII letters and digits with no separator, kept
    /// once in first-seen order. [`Lexicon::from_packages`] derives terms from
    /// package specifiers.
    #[must_use]
    pub fn with(terms: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let mut kept = Vec::new();
        for term in terms.into_iter().map(Into::into) {
            if !kept.contains(&term) {
                kept.push(term);
            }
        }
        Self { terms: kept }
    }

    /// Returns the lexicon with the terms of the package `specifiers` beside the fixed names.
    ///
    /// A specifier yields the names a text could spell only on purpose, each
    /// once:
    ///
    /// - the scope of a scoped package, which always names a vendor or a
    ///   product: `@azure`, `@aws-sdk`, `@nestjs`
    /// - a compound name, joined: `keyvault-secrets`, `confluent_kafka`,
    ///   `socket.io`, which a text names only by spelling both parts
    /// - a lone name that is coined rather than plain: one holding a digit,
    ///   led by `py`, ending in `js`, `db`, or `mq`, or holding `sql`
    ///
    /// A lone plain word (`requests`, `events`, `express`) yields nothing
    /// because it would turn the domain's own words into findings. The fixed
    /// names hold the products among them. A path beneath the package
    /// (`lodash/fp`) is ignored.
    ///
    /// # Examples
    ///
    /// ```
    /// use emery_lexicon::Lexicon;
    ///
    /// assert_eq!(
    ///     Lexicon::from_packages(["@azure/keyvault-secrets", "@nestjs/common", "boto3"]),
    ///     Lexicon::with(["azure", "keyvaultsecrets", "nestjs", "boto3"])
    /// );
    /// assert_eq!(Lexicon::from_packages(["kafkajs"]), Lexicon::with(["kafkajs"]));
    /// assert_eq!(Lexicon::from_packages(["confluent_kafka"]), Lexicon::with(["confluentkafka"]));
    /// assert_eq!(Lexicon::from_packages(["requests", "express"]), Lexicon::default());
    /// ```
    #[must_use]
    pub fn from_packages(specifiers: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        Self::with(specifiers.into_iter().flat_map(package_terms))
    }

    /// Returns the technology `text` names, once each, in the order the text reaches them.
    ///
    /// `text` is split into [`words`], so `useConfluentKafkaConfig`,
    /// `KEYVAULT_HOST`, and `key-vault.refuse-empty` read as the words they
    /// join. A word, or up to three consecutive words joined, that matches an
    /// entry of the lexicon names that entry. Nothing is returned for a text
    /// naming none.
    ///
    /// # Examples
    ///
    /// ```
    /// use emery_lexicon::Lexicon;
    ///
    /// let lexicon = Lexicon::default();
    /// assert_eq!(lexicon.names("useConfluentKafkaConfig"), ["confluent", "kafka"]);
    /// assert_eq!(lexicon.names("KEYVAULT_HOST"), ["keyvault"]);
    /// assert_eq!(lexicon.names("key-vault.refuse-empty"), ["keyvault"]);
    /// assert!(lexicon.names("Each converted event is published to the outbound stream.").is_empty());
    /// ```
    #[must_use]
    pub fn names(&self, text: &str) -> Vec<String> {
        let words = words(text);
        let mut named: Vec<String> = Vec::new();
        let mut start = 0;
        while start < words.len() {
            // find the longest run from here that spells a name
            let mut joined = String::new();
            let mut hit: Option<(usize, &str)> = None;
            for (length, word) in (1..=RUN).zip(&words[start..]) {
                if length > 1 && word.len() < 2 {
                    break;
                }
                joined.push_str(word);
                if let Some(name) = self.lookup(&joined) {
                    hit = Some((length, name));
                }
            }

            // record the matched name and advance past its words
            match hit {
                Some((length, name)) => {
                    if !named.iter().any(|seen| seen == name) {
                        named.push(name.to_owned());
                    }
                    start += length;
                }
                None => start += 1,
            }
        }
        named
    }

    fn lookup(&self, candidate: &str) -> Option<&str> {
        TECHNOLOGY
            .iter()
            .copied()
            .find(|&name| name == candidate)
            .or_else(|| self.terms.iter().map(String::as_str).find(|&term| term == candidate))
    }
}

/// Returns the words of `text`, lowercased, split at its separators and camel-case boundaries.
///
/// A word is a run of ASCII letters and digits. A run splits again where a
/// lowercase letter or a digit meets an uppercase one, and where an uppercase
/// run meets its last letter's lowercase successor, so `IORedis` is `io`,
/// `redis`. Nothing is returned for a text of separators alone.
///
/// # Examples
///
/// ```
/// use emery_lexicon::words;
///
/// assert_eq!(words("useConfluentKafkaConfig"), ["use", "confluent", "kafka", "config"]);
/// assert_eq!(words("KEYVAULT_HOST"), ["keyvault", "host"]);
/// assert_eq!(words("HTTPServer v2"), ["http", "server", "v2"]);
/// assert!(words("--").is_empty());
/// ```
#[must_use]
pub fn words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut prev_upper = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if !c.is_ascii_alphanumeric() {
            if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
            prev_upper = false;
            continue;
        }
        let upper = c.is_ascii_uppercase();
        if upper
            && !word.is_empty()
            && (!prev_upper || chars.peek().is_some_and(char::is_ascii_lowercase))
        {
            words.push(std::mem::take(&mut word));
        }
        word.push(c.to_ascii_lowercase());
        prev_upper = upper;
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}

// The terms of one package specifier, spelled as a fixed name is, under the
// rules `Lexicon::from_packages` states.
fn package_terms(specifier: impl AsRef<str>) -> impl Iterator<Item = String> {
    let specifier = specifier.as_ref();
    let (scope, rest) = specifier.strip_prefix('@').map_or((None, specifier), |scoped| {
        let (scope, rest) = scoped.split_once('/').unwrap_or((scoped, ""));
        (Some(scope), rest)
    });
    let name = rest.split('/').next().unwrap_or(rest);
    scope.and_then(scope_term).into_iter().chain(name_term(name))
}

// The scope of a scoped package is a vendor's or a product's whatever word it is.
fn scope_term(scope: &str) -> Option<String> {
    let parts = parts(scope);
    (!parts.is_empty()).then(|| parts.concat().to_ascii_lowercase())
}

// A package name counts when a text could spell it only on purpose: a
// compound name, or a lone word that is coined rather than plain.
fn name_term(name: &str) -> Option<String> {
    match parts(name).as_slice() {
        [] => None,
        [one] => {
            let term = one.to_ascii_lowercase();
            is_coined(&term).then_some(term)
        }
        many => Some(many.concat().to_ascii_lowercase()),
    }
}

const COINED_LEADS: &[&str] = &["py"];
const COINED_TAILS: &[&str] = &["js", "db", "mq"];
const COINED_PARTS: &[&str] = &["sql"];

// What marks a lone word of a package name as coined rather than plain:
// `boto3`, `kafkajs`, `mongodb`, `bullmq`, `sqlalchemy`, `pymongo`.
fn is_coined(word: &str) -> bool {
    word.bytes().any(|byte| byte.is_ascii_digit())
        || COINED_LEADS.iter().any(|lead| word.starts_with(lead) && word.len() > lead.len())
        || COINED_TAILS.iter().any(|tail| word.ends_with(tail) && word.len() > tail.len())
        || COINED_PARTS.iter().any(|part| word.contains(part))
}

// The parts of a package name: runs of letters and digits between its
// separators, with no camel-case split since a package name has none.
fn parts(name: &str) -> Vec<&str> {
    name.split(|c: char| !c.is_ascii_alphanumeric()).filter(|part| !part.is_empty()).collect()
}

// The fixed names of vendors, products, protocols, frameworks, and hosts.
//
// Every entry is lowercase letters and digits with no separator.
// `Key Vault`, `key-vault`, and `KeyVault` all spell `keyvault`.
//
// A name that can also be a plain word of a domain (`express`, `spring`,
// `rabbit`, `vault`) is left out; where the product has a fuller spelling,
// that is the entry (`expressjs`, `springboot`, `rabbitmq`, `hashicorp`).
const TECHNOLOGY: &[&str] = &[
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{Lexicon, TECHNOLOGY, words};

    // Every fixed entry is non-empty, lowercase ASCII letters and digits with
    // no separator, and listed once.
    #[test]
    fn fixed_table_invariants() {
        let mut seen = BTreeSet::new();
        for &entry in TECHNOLOGY {
            assert!(!entry.is_empty(), "empty lexicon entry");
            assert!(
                entry.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()),
                "`{entry}` must be lowercase ASCII letters and digits without separators"
            );
            assert!(seen.insert(entry), "`{entry}` is listed twice in TECHNOLOGY");
        }
    }

    #[test]
    fn package_specifiers() {
        let lexicon = Lexicon::from_packages([
            "@azure/keyvault-secrets/subpath",
            "@aws-sdk/client-s3",
            "@/empty",
            "@",
            "socket.io/client",
            "pymongo",
            "py",
            "bullmq",
            "mq",
            "sqlalchemy",
            "requests",
            "boto3",
            "boto3",
        ]);
        assert_eq!(
            lexicon,
            Lexicon::with([
                "azure",
                "keyvaultsecrets",
                "awssdk",
                "clients3",
                "socketio",
                "pymongo",
                "bullmq",
                "sqlalchemy",
                "boto3",
            ])
        );
    }

    // Longest match wins (`eventhubs` over `eventhub`), a single-character
    // trailing word (`ek s`) does not complete a multi-word entry, and each
    // matched name is returned once in first-reached order.
    #[test]
    fn longest_and_deduplicated_matches() {
        let lexicon = Lexicon::from_packages(["@azure/keyvault-secrets"]);
        assert_eq!(
            lexicon.names("Event Hubs and Key Vault Secrets and Event Hub and Key Vault"),
            ["eventhubs", "keyvaultsecrets", "eventhub", "keyvault"]
        );
        assert_eq!(lexicon.names("ek s"), Vec::<String>::new());
    }

    #[test]
    fn camel_and_acronym_words() {
        assert_eq!(words("IORedis2Client"), ["io", "redis2", "client"]);
        assert_eq!(words("v2Beta"), ["v2", "beta"]);
    }
}
