use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VariableBlockWarning
{
    InvalidFinalArgument,
    RangeCoversNonExistentVariables,
}

impl fmt::Display for VariableBlockWarning
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::InvalidFinalArgument =>
            {
                formatter.write_str("cmu_bdd_new_var_block: invalid final argument")
            }
            Self::RangeCoversNonExistentVariables =>
            {
                formatter.write_str("cmu_bdd_new_var_block: range covers non-existent variables")
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VariableBlockError
{
    EmptyManager,
    IllegalBlockOverlap,
    InvalidVariableIndex
    {
        index: usize,
        variable_count: usize,
    },
}

impl fmt::Display for VariableBlockError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::EmptyManager => formatter.write_str(
                "cmu_bdd_new_var_block: manager has no variables"
            ),
            Self::IllegalBlockOverlap => formatter.write_str("add_block: illegal block overlap"),
            Self::InvalidVariableIndex
            {
                index,
                variable_count,
            } => write!(
                formatter,
                "cmu_bdd_new_var_block: variable index {index} is outside {variable_count} variables"
            ),
        }
    }
}

impl std::error::Error for VariableBlockError
{
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariableBlock
{
    reorderable: bool,
    first_index: usize,
    last_index: usize,
    children: Vec<VariableBlock>,
}

impl VariableBlock
{
    pub fn leaf(index: usize) -> Self
    {
        Self
        {
            reorderable: false,
            first_index: index,
            last_index: index,
            children: Vec::new(),
        }
    }

    pub fn new(
        first_index: usize,
        last_index: usize,
        reorderable: bool,
        children: Vec<Self>,
    ) -> Self
    {
        Self
        {
            reorderable,
            first_index,
            last_index,
            children,
        }
    }

    pub fn balanced(variable_count: usize) -> Result<Self, VariableBlockError>
    {
        if variable_count == 0
        {
            return Err(VariableBlockError::EmptyManager);
        }

        Ok(Self::new(
            0,
            variable_count - 1,
            true,
            (0..variable_count).map(Self::leaf).collect(),
        ))
    }

    pub fn first_index(&self) -> usize
    {
        self.first_index
    }

    pub fn last_index(&self) -> usize
    {
        self.last_index
    }

    pub fn reorderable(&self) -> bool
    {
        self.reorderable
    }

    pub fn children(&self) -> &[Self]
    {
        &self.children
    }

    pub fn variable_count(&self) -> usize
    {
        self.last_index - self.first_index + 1
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariableBlockCreation
{
    block: VariableBlock,
    warnings: Vec<VariableBlockWarning>,
}

impl VariableBlockCreation
{
    pub fn block(&self) -> &VariableBlock
    {
        &self.block
    }

    pub fn warnings(&self) -> &[VariableBlockWarning]
    {
        &self.warnings
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariableBlockManager
{
    variable_count: usize,
    super_block: VariableBlock,
}

impl VariableBlockManager
{
    pub fn new(variable_count: usize) -> Result<Self, VariableBlockError>
    {
        Ok(Self
        {
            variable_count,
            super_block: VariableBlock::balanced(variable_count)?,
        })
    }

    pub fn variable_count(&self) -> usize
    {
        self.variable_count
    }

    pub fn super_block(&self) -> &VariableBlock
    {
        &self.super_block
    }

    pub fn new_variable_block(
        &mut self,
        variable_index: usize,
        requested_count: isize,
    ) -> Result<VariableBlockCreation, VariableBlockError>
    {
        if variable_index >= self.variable_count
        {
            return Err(VariableBlockError::InvalidVariableIndex
            {
                index: variable_index,
                variable_count: self.variable_count,
            });
        }

        let mut warnings = Vec::new();
        let mut count = requested_count;

        if count <= 0
        {
            warnings.push(VariableBlockWarning::InvalidFinalArgument);
            count = 1;
        }

        let first_index = variable_index;
        let mut last_index = first_index.saturating_add(count as usize).saturating_sub(1);

        if last_index >= self.variable_count
        {
            warnings.push(VariableBlockWarning::RangeCoversNonExistentVariables);
            last_index = self.variable_count - 1;
        }

        let mut block = VariableBlock::new(first_index, last_index, false, Vec::new());
        add_block(&mut self.super_block, &mut block)?;

        Ok(VariableBlockCreation
        {
            block,
            warnings,
        })
    }
}

pub fn find_block(block: &VariableBlock, index: usize) -> usize
{
    let mut low = 0;
    let mut high = block.children.len();

    while low < high
    {
        let mid = low + (high - low) / 2;
        let child = &block.children[mid];

        if child.first_index <= index && child.last_index >= index
        {
            return mid;
        }

        if child.first_index > index
        {
            high = mid;
        }
        else
        {
            low = mid + 1;
        }
    }

    low
}

fn add_block(
    parent: &mut VariableBlock,
    block: &mut VariableBlock,
) -> Result<(), VariableBlockError>
{
    if parent.children.is_empty()
    {
        parent.children.push(block.clone());
        block.children.clear();
        return Ok(());
    }

    let start_index = find_block(parent, block.first_index);
    let end_index = find_block(parent, block.last_index);

    if start_index == end_index
    {
        return add_block(&mut parent.children[start_index], block);
    }

    let start = &parent.children[start_index];
    let end = &parent.children[end_index];

    if start.first_index != block.first_index || end.last_index != block.last_index
    {
        return Err(VariableBlockError::IllegalBlockOverlap);
    }

    block.children = parent.children[start_index..=end_index].to_vec();
    parent
        .children
        .splice(start_index..=end_index, [block.clone()]);

    Ok(())
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn new_variable_block_groups_adjacent_leaves()
    {
        let mut manager = VariableBlockManager::new(4).unwrap();
        let result = manager.new_variable_block(1, 2).unwrap();

        assert!(result.warnings().is_empty());
        assert_eq!(result.block().first_index(), 1);
        assert_eq!(result.block().last_index(), 2);
        assert_eq!(
            manager.super_block().children(),
            &[
                VariableBlock::leaf(0),
                VariableBlock::new(
                    1,
                    2,
                    false,
                    vec![VariableBlock::leaf(1), VariableBlock::leaf(2)]
                ),
                VariableBlock::leaf(3),
            ]
        );
    }

    #[test]
    fn nested_range_is_added_inside_existing_block()
    {
        let mut manager = VariableBlockManager::new(5).unwrap();

        manager.new_variable_block(1, 3).unwrap();
        manager.new_variable_block(2, 1).unwrap();

        let outer = &manager.super_block().children()[1];
        assert_eq!(outer.first_index(), 1);
        assert_eq!(outer.last_index(), 3);
        assert_eq!(
            outer.children(),
            &[
                VariableBlock::leaf(1),
                VariableBlock::new(2, 2, false, vec![VariableBlock::leaf(2)]),
                VariableBlock::leaf(3),
            ]
        );
    }

    #[test]
    fn invalid_count_warns_and_uses_one_variable()
    {
        let mut manager = VariableBlockManager::new(2).unwrap();
        let result = manager.new_variable_block(1, 0).unwrap();

        assert_eq!(
            result.warnings(),
            &[VariableBlockWarning::InvalidFinalArgument]
        );
        assert_eq!(result.block().first_index(), 1);
        assert_eq!(result.block().last_index(), 1);
    }

    #[test]
    fn range_past_manager_warns_and_clamps_to_last_variable()
    {
        let mut manager = VariableBlockManager::new(3).unwrap();
        let result = manager.new_variable_block(1, 10).unwrap();

        assert_eq!(
            result.warnings(),
            &[VariableBlockWarning::RangeCoversNonExistentVariables]
        );
        assert_eq!(result.block().first_index(), 1);
        assert_eq!(result.block().last_index(), 2);
    }

    #[test]
    fn huge_range_past_manager_warns_and_clamps_to_last_variable()
    {
        let mut manager = VariableBlockManager::new(3).unwrap();
        let result = manager.new_variable_block(1, isize::MAX).unwrap();

        assert_eq!(
            result.warnings(),
            &[VariableBlockWarning::RangeCoversNonExistentVariables]
        );
        assert_eq!(result.block().first_index(), 1);
        assert_eq!(result.block().last_index(), 2);
    }

    #[test]
    fn partial_overlap_returns_error()
    {
        let mut manager = VariableBlockManager::new(4).unwrap();

        manager.new_variable_block(1, 2).unwrap();
        let error = manager.new_variable_block(0, 2).unwrap_err();

        assert_eq!(error, VariableBlockError::IllegalBlockOverlap);
    }

    #[test]
    fn find_block_returns_matching_child_or_insertion_point()
    {
        let block = VariableBlock::new(
            0,
            5,
            true,
            vec![
                VariableBlock::leaf(0),
                VariableBlock::new(2, 3, false, Vec::new()),
                VariableBlock::leaf(5),
            ],
        );

        assert_eq!(find_block(&block, 0), 0);
        assert_eq!(find_block(&block, 3), 1);
        assert_eq!(find_block(&block, 4), 2);
        assert_eq!(find_block(&block, 6), 3);
    }
}
