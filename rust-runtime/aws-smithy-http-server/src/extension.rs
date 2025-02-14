/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0.
 */

// This code was copied and then modified from Tokio's Axum.

/* Copyright (c) 2021 Tower Contributors
 *
 * Permission is hereby granted, free of charge, to any
 * person obtaining a copy of this software and associated
 * documentation files (the "Software"), to deal in the
 * Software without restriction, including without
 * limitation the rights to use, copy, modify, merge,
 * publish, distribute, sublicense, and/or sell copies of
 * the Software, and to permit persons to whom the Software
 * is furnished to do so, subject to the following
 * conditions:
 *
 * The above copyright notice and this permission notice
 * shall be included in all copies or substantial portions
 * of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
 * ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
 * TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
 * PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
 * SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
 * CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
 * OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
 * IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
 * DEALINGS IN THE SOFTWARE.
 */

//! Extension types.
//!
//! Extension types are types that are stored in and extracted from _both_ requests and
//! responses.
//!
//! There is only one _generic_ extension type _for requests_, [`Extension`].
//!
//! On the other hand, the server SDK uses multiple concrete extension types for responses in order
//! to store a variety of information, like the operation that was executed, the operation error
//! that got returned, or the runtime error that happened, among others. The information stored in
//! these types may be useful to [`tower::Layer`]s that post-process the response: for instance, a
//! particular metrics layer implementation might want to emit metrics about the number of times an
//! an operation got executed.
//!
//! [extensions]: https://docs.rs/http/latest/http/struct.Extensions.html

use axum_core::extract::RequestParts;
use std::ops::Deref;

/// Extension type used to store information about Smithy operations in HTTP responses.
/// This extension type is set when it has been correctly determined that the request should be
/// routed to a particular operation. The operation handler might not even get invoked because the
/// request fails to deserialize into the modeled operation input.
#[derive(Debug, Clone)]
pub struct OperationExtension {
    /// Smithy model namespace.
    namespace: &'static str,
    /// Smithy operation name.
    operation_name: &'static str,
}

impl OperationExtension {
    /// Creates a new `OperationExtension`.
    pub fn new(namespace: &'static str, operation_name: &'static str) -> Self {
        Self {
            namespace,
            operation_name,
        }
    }

    /// Returns the current operation formatted as `<namespace>#<operation_name>`.
    pub fn operation(&self) -> String {
        format!("{}#{}", self.namespace, self.operation_name)
    }
}

/// Extension type used to store the type of user-modeled error returned by an operation handler.
/// These are modeled errors, defined in the Smithy model.
#[derive(Debug, Clone)]
pub struct ModeledErrorExtension(&'static str);
impl_extension_new_and_deref!(ModeledErrorExtension);

/// Extension type used to store the _name_ of the [`crate::runtime_error::RuntimeError`] that
/// occurred during request handling (see [`crate::runtime_error::RuntimeErrorKind::name`]).
/// These are _unmodeled_ errors; the operation handler was not invoked.
#[derive(Debug, Clone)]
pub struct RuntimeErrorExtension(String);

impl RuntimeErrorExtension {
    /// Creates a new `RuntimeErrorExtension`.
    pub fn new(value: String) -> RuntimeErrorExtension {
        RuntimeErrorExtension(value)
    }
}

impl Deref for RuntimeErrorExtension {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Generic extension type stored in and extracted from [request extensions].
///
/// This is commonly used to share state across handlers.
///
/// If the extension is missing it will reject the request with a `500 Internal
/// Server Error` response.
///
/// [request extensions]: https://docs.rs/http/latest/http/struct.Extensions.html
#[derive(Debug, Clone)]
pub struct Extension<T>(pub T);

impl<T> Deref for Extension<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Extract an [`Extension`] from a request.
/// This is essentially the implementation of `FromRequest` for `Extension`, but with a
/// protocol-agnostic rejection type. The actual code-generated implementation simply delegates to
/// this function and converts the rejection type into a [`crate::runtime_error::RuntimeError`].
pub async fn extract_extension<T, B>(
    req: &mut RequestParts<B>,
) -> Result<Extension<T>, crate::rejection::RequestExtensionNotFoundRejection>
where
    T: Clone + Send + Sync + 'static,
    B: Send,
{
    let value = req
        .extensions()
        .ok_or(crate::rejection::RequestExtensionNotFoundRejection::ExtensionsAlreadyExtracted)?
        .get::<T>()
        .ok_or_else(|| {
            crate::rejection::RequestExtensionNotFoundRejection::MissingExtension(format!(
                "Extension of type `{}` was not found. Perhaps you forgot to add it?",
                std::any::type_name::<T>()
            ))
        })
        .map(|x| x.clone())?;

    Ok(Extension(value))
}
