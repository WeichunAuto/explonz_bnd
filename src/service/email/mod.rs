use askama::Template;

#[derive(Template)]
#[template(path = "verify_email.html")]
pub struct VerifyEmailTemplate<'a> {
    pub code: &'a str,
    pub expire_minutes: u32,
}
