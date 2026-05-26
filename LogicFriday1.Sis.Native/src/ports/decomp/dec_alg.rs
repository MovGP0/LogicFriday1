use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecursiveDecompositionError<E>
{
    SubstitutionRejected,
    Backend(E),
}

impl<E> RecursiveDecompositionError<E>
{
    pub fn backend(error: E) -> Self
    {
        Self::Backend(error)
    }
}

impl<E> fmt::Display for RecursiveDecompositionError<E>
where
    E: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::SubstitutionRejected =>
            {
                f.write_str("decomposition divisor could not be substituted")
            }
            Self::Backend(error) => write!(f, "{error}"),
        }
    }
}

impl<E> Error for RecursiveDecompositionError<E>
where
    E: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)>
    {
        match self
        {
            Self::SubstitutionRejected => None,
            Self::Backend(error) => Some(error),
        }
    }
}

pub type RecursiveDecompositionResult<T, E> = Result<T, RecursiveDecompositionError<E>>;

pub trait RecursiveDecomposition
{
    type Node;
    type Error;

    fn generate_divisor(
        &mut self,
        node: &Self::Node,
    ) -> Result<Option<Self::Node>, Self::Error>;

    fn substitute_divisor(
        &mut self,
        node: &mut Self::Node,
        divisor: &Self::Node,
    ) -> Result<bool, Self::Error>;
}

pub fn decompose_recursively<D>(
    decomposition: &mut D,
    node: D::Node,
) -> RecursiveDecompositionResult<Vec<D::Node>, D::Error>
where
    D: RecursiveDecomposition,
{
    let mut node = node;
    let Some(divisor) = decomposition
        .generate_divisor(&node)
        .map_err(RecursiveDecompositionError::Backend)? else
    {
        return Ok(vec![node]);
    };

    if !decomposition
        .substitute_divisor(&mut node, &divisor)
        .map_err(RecursiveDecompositionError::Backend)?
    {
        return Err(RecursiveDecompositionError::SubstitutionRejected);
    }

    let mut node_decomposition = decompose_recursively(decomposition, node)?;
    let mut divisor_decomposition = decompose_recursively(decomposition, divisor)?;
    node_decomposition.append(&mut divisor_decomposition);

    Ok(node_decomposition)
}

#[cfg(test)]
mod tests
{
    use super::*;
    use std::collections::HashMap;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TestNode
    {
        name: String,
        divisors: Vec<String>,
    }

    impl TestNode
    {
        fn new(name: &str, divisors: &[&str]) -> Self
        {
            Self
            {
                name: name.to_owned(),
                divisors: divisors.iter().map(|divisor| (*divisor).to_owned()).collect(),
            }
        }
    }

    #[derive(Default)]
    struct TestDecomposition
    {
        divisor_tables: HashMap<String, Vec<String>>,
        reject_substitution: bool,
    }

    impl TestDecomposition
    {
        fn with_node(mut self, node: &str, divisors: &[&str]) -> Self
        {
            self.divisor_tables.insert(
                node.to_owned(),
                divisors.iter().map(|divisor| (*divisor).to_owned()).collect(),
            );

            self
        }
    }

    impl RecursiveDecomposition for TestDecomposition
    {
        type Node = TestNode;
        type Error = String;

        fn generate_divisor(
            &mut self,
            node: &Self::Node,
        ) -> Result<Option<Self::Node>, Self::Error>
        {
            Ok(node.divisors.first().map(|name|
            {
                let divisors = self
                    .divisor_tables
                    .get(name)
                    .cloned()
                    .unwrap_or_default();

                TestNode
                {
                    name: name.clone(),
                    divisors,
                }
            }))
        }

        fn substitute_divisor(
            &mut self,
            node: &mut Self::Node,
            divisor: &Self::Node,
        ) -> Result<bool, Self::Error>
        {
            if self.reject_substitution
            {
                return Ok(false);
            }

            let Some(index) = node
                .divisors
                .iter()
                .position(|candidate| candidate == &divisor.name) else
            {
                return Ok(false);
            };

            node.divisors.remove(index);
            node.name = format!("{}-without-{}", node.name, divisor.name);

            Ok(true)
        }
    }

    #[test]
    fn node_without_divisor_returns_singleton()
    {
        let mut decomposition = TestDecomposition::default();
        let node = TestNode::new("f", &[]);

        let result = decompose_recursively(&mut decomposition, node).unwrap();

        assert_eq!(result, vec![TestNode::new("f", &[])]);
    }

    #[test]
    fn recursively_decomposes_remainder_before_divisor()
    {
        let mut decomposition = TestDecomposition::default()
            .with_node("g", &["h"])
            .with_node("h", &[]);
        let node = TestNode::new("f", &["g"]);

        let result = decompose_recursively(&mut decomposition, node).unwrap();
        let names: Vec<_> = result.iter().map(|node| node.name.as_str()).collect();

        assert_eq!(names, vec!["f-without-g", "g-without-h", "h"]);
    }

    #[test]
    fn rejected_substitution_is_reported()
    {
        let mut decomposition = TestDecomposition
        {
            reject_substitution: true,
            ..Default::default()
        };
        let node = TestNode::new("f", &["g"]);

        let error = decompose_recursively(&mut decomposition, node).unwrap_err();

        assert_eq!(error, RecursiveDecompositionError::SubstitutionRejected);
        assert_eq!(
            error.to_string(),
            "decomposition divisor could not be substituted"
        );
    }

    #[test]
    fn backend_error_from_divisor_generation_is_preserved()
    {
        struct FailingDecomposition;

        impl RecursiveDecomposition for FailingDecomposition
        {
            type Node = TestNode;
            type Error = &'static str;

            fn generate_divisor(
                &mut self,
                _node: &Self::Node,
            ) -> Result<Option<Self::Node>, Self::Error>
            {
                Err("failed")
            }

            fn substitute_divisor(
                &mut self,
                _node: &mut Self::Node,
                _divisor: &Self::Node,
            ) -> Result<bool, Self::Error>
            {
                unreachable!()
            }
        }

        let mut decomposition = FailingDecomposition;
        let node = TestNode::new("f", &[]);

        let error = decompose_recursively(&mut decomposition, node).unwrap_err();

        assert_eq!(error, RecursiveDecompositionError::Backend("failed"));
    }
}
