//! The application's test support: what an update test reads of the effects a message makes, the
//! answer it gives an ask, and the keys it loads.

use faraday_app_test::{Kind, TestView, answer, seen};
use faraday_kernel::{Effects, Job, JobKind, Message};
use faraday_port::{Answer, Ask, Order, Request};
use faraday_testkit::test_seeds;

#[derive(Debug, PartialEq, Eq)]
enum Msg {
    First,
    Second,
    Worked(u32),
    Free(u32, Option<u32>),
}

impl Message for Msg {
    fn kind(&self) -> &'static str {
        "test.msg"
    }

    fn public(&self) -> Option<&dyn core::fmt::Debug> {
        Some(self)
    }
}

/// An ask for the free memory, as a feature's own request type would be.
struct Probe;

impl From<Probe> for Ask {
    fn from(_probe: Probe) -> Ask {
        Ask::Memory
    }
}

impl Request for Probe {
    type Answer = Option<u32>;

    fn take(answer: Answer) -> Result<Option<u32>, Answer> {
        match answer {
            Answer::Memory(free) => Ok(free),
            other => Err(other),
        }
    }

    fn fault() -> Option<u32> {
        None
    }
}

fn free(context: u32, free: Option<u32>) -> Msg {
    Msg::Free(context, free)
}

/// Invariant: An update test sees the effects a message makes in the order the update returned them, and in each kind's own order.
/// Regression: `seen` sorts the effects by kind and loses the order they were returned in.
/// Owner: app
#[test]
fn effects_are_seen_in_the_order_they_were_made() {
    let fx: Effects<Msg, Msg> = Effects::none()
        .order(Order::Vibrate { ms: 5 })
        .then(Msg::First)
        .ask(Probe, 7, free)
        .job(Job::new(JobKind("test.job"), 41u32, |n, _compute| {
            Msg::Worked(n + 1)
        }))
        .out(Msg::Second)
        .then(Msg::Second);
    let seen = seen(fx);
    assert_eq!(
        seen.order,
        [
            Kind::Order,
            Kind::Then,
            Kind::Ask,
            Kind::Job,
            Kind::Out,
            Kind::Then
        ]
    );
    assert_eq!(seen.thens, [Msg::First, Msg::Second]);
    assert_eq!(seen.outs, [Msg::Second]);
    assert_eq!(seen.orders.len(), 1);
}

/// Invariant: An answer given to an ask reaches the feature as the message its reply builds from the ask's own context.
/// Regression: `answer` drops the context the ask kept, or hands the reply another ask's answer.
/// Owner: app
#[test]
fn an_answer_reaches_the_asker_with_its_context() {
    let fx: Effects<Msg, Msg> = Effects::none().ask(Probe, 7, free);
    let mut seen = seen(fx);
    let ask = seen.asks.remove(0);
    assert_eq!(
        answer(ask, Answer::Memory(Some(512))),
        Msg::Free(7, Some(512))
    );
}

/// Invariant: A job's work runs inline and ends in the message the feature handles next.
/// Regression: `work` returns the job's input, not the message its work made.
/// Owner: app
#[test]
fn a_job_runs_inline_to_its_message() {
    let fx: Effects<Msg, Msg> =
        Effects::none().job(Job::new(JobKind("test.job"), 41u32, |n, _compute| {
            Msg::Worked(n + 1)
        }));
    let mut seen = seen(fx);
    let (kind, message) = faraday_app_test::work(seen.jobs.remove(0));
    assert_eq!(kind, JobKind("test.job"));
    assert_eq!(message, Msg::Worked(42));
}

/// Invariant: A key given to a test view with more than one key keeps every key given.
/// Regression: `with_key` rebuilds the session from the last seed only.
/// Owner: app
#[test]
fn a_view_with_two_keys_holds_both() {
    let seeds = test_seeds();
    let view = TestView::device().with_key(seeds[0]).with_key(seeds[1]);
    assert_eq!(view.ctx().view.session.keys().len(), 2);
}
