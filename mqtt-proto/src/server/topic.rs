pub trait TopicProvider: Clone {}

#[derive(Clone, Debug, Default)]
pub struct InMemoryTopicProvider {}

impl TopicProvider for InMemoryTopicProvider {}
