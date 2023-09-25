use async_trait::async_trait;
use axum::{
    extract::FromRequestParts,
    http::{header::REFERER, request::Parts},
};
use oxi_axum_helpers::{RespErr, RespErrCtx, RespErrExt, Status};
use url::Url;

use crate::states::InnerAppState;

const MAX_HEADER_VALUE: usize = 255;

pub struct ReferrerDomain(pub Option<String>);

#[async_trait]
impl FromRequestParts<&'static InnerAppState> for ReferrerDomain {
    type Rejection = RespErr;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &&'static InnerAppState,
    ) -> Result<Self, Self::Rejection> {
        let Some(value) = parts.headers.get(REFERER) else {
            return Ok(Self(None));
        };

        if value.len() > MAX_HEADER_VALUE {
            return Err(RespErr::new(Status::BadRequest).user_msg(format!(
                "REFERER header value is longer than {MAX_HEADER_VALUE}"
            )));
        }

        let referrer = value
            .to_str()
            .ctx(Status::BadRequest)
            .user_msg("The header value is not a valid string!")?;

        if referrer.starts_with(&state.tracked_origin) {
            // Don't count the tracked domain as a referrer domain.
            return Ok(Self(None));
        }

        let url = Url::parse(referrer)
            .ctx(Status::BadRequest)
            .user_msg("Failed to parse the referrer as a valid URL")?;
        let domain = url
            .domain()
            .ctx(Status::BadRequest)
            .user_msg("Failed to get the referrer domain!")?
            .to_owned();

        Ok(Self(Some(domain)))
    }
}
