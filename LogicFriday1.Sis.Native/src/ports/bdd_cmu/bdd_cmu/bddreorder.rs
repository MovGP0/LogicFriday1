use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DynamicReorderMethod
{
    StableWindow3,
    Sift,
    Hybrid,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BddReorderError
{
    EmptyBlock,
    EmptyVariableOrder,
    InvalidBlockRange
    {
        first_index: usize,
        last_index: usize,
    },
    InvalidChildIndex
    {
        child_index: usize,
        child_count: usize,
    },
    InvalidVariableIndex
    {
        index: usize,
        variable_count: usize,
    },
    MissingAdjacentVariable
    {
        left_index: usize,
        right_index: usize,
    },
    LevelWidthMismatch
    {
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for BddReorderError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::EmptyBlock => formatter.write_str("BDD reorder block must contain variables"),
            Self::EmptyVariableOrder => formatter.write_str("BDD reorder variable order is empty"),
            Self::InvalidBlockRange
            {
                first_index,
                last_index,
            } => write!(
                formatter,
                "BDD reorder block has invalid range {first_index}..={last_index}"
            ),
            Self::InvalidChildIndex
            {
                child_index,
                child_count,
            } => write!(
                formatter,
                "BDD reorder child index {child_index} is outside {child_count} children"
            ),
            Self::InvalidVariableIndex
            {
                index,
                variable_count,
            } => write!(
                formatter,
                "BDD reorder variable index {index} is outside {variable_count} variables"
            ),
            Self::MissingAdjacentVariable
            {
                left_index,
                right_index,
            } => write!(
                formatter,
                "BDD reorder variables {left_index} and {right_index} are not adjacent"
            ),
            Self::LevelWidthMismatch
            {
                expected,
                actual,
            } => write!(
                formatter,
                "BDD reorder level width table has {actual} entries, expected {expected}"
            ),
        }
    }
}

impl std::error::Error for BddReorderError
{
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariableBlock
{
    id: usize,
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
            id: 0,
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
    ) -> Result<Self, BddReorderError>
    {
        if first_index > last_index
        {
            return Err(BddReorderError::InvalidBlockRange
            {
                first_index,
                last_index,
            });
        }

        Ok(Self
        {
            id: 0,
            reorderable,
            first_index,
            last_index,
            children,
        })
    }

    pub fn balanced(variable_count: usize) -> Result<Self, BddReorderError>
    {
        if variable_count == 0
        {
            return Err(BddReorderError::EmptyBlock);
        }

        let children = (0..variable_count).map(Self::leaf).collect();

        Self::new(0, variable_count - 1, true, children)
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

    pub fn set_reorderable(&mut self, reorderable: bool)
    {
        self.reorderable = reorderable;
    }

    pub fn children(&self) -> &[Self]
    {
        &self.children
    }

    pub fn children_mut(&mut self) -> &mut [Self]
    {
        &mut self.children
    }

    pub fn variable_count(&self) -> usize
    {
        self.last_index - self.first_index + 1
    }

    fn assign_ids(&mut self, next_id: &mut usize)
    {
        self.id = *next_id;
        *next_id += 1;

        for child in &mut self.children
        {
            child.assign_ids(next_id);
        }
    }

    fn shift_indices(&mut self, delta: isize)
    {
        self.first_index = self.first_index.saturating_add_signed(delta);
        self.last_index = self.last_index.saturating_add_signed(delta);

        for child in &mut self.children
        {
            child.shift_indices(delta);
        }
    }

    fn validate(&self, variable_count: usize) -> Result<(), BddReorderError>
    {
        if self.first_index > self.last_index
        {
            return Err(BddReorderError::InvalidBlockRange
            {
                first_index: self.first_index,
                last_index: self.last_index,
            });
        }

        if self.last_index >= variable_count
        {
            return Err(BddReorderError::InvalidVariableIndex
            {
                index: self.last_index,
                variable_count,
            });
        }

        for child in &self.children
        {
            child.validate(variable_count)?;
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BddReorderStats
{
    pub node_count: usize,
    pub adjacent_variable_exchanges: usize,
    pub block_exchanges: usize,
}

#[derive(Clone, Debug)]
pub struct BddReorderManager
{
    root: VariableBlock,
    variable_order: Vec<usize>,
    level_widths: Vec<usize>,
    node_count: usize,
    method: Option<DynamicReorderMethod>,
    hybrid_size_factor: f64,
    adjacent_variable_exchanges: usize,
    block_exchanges: usize,
}

impl BddReorderManager
{
    pub fn new(mut root: VariableBlock, node_count: usize) -> Result<Self, BddReorderError>
    {
        let variable_count = root.variable_count();

        if variable_count == 0
        {
            return Err(BddReorderError::EmptyVariableOrder);
        }

        root.validate(variable_count)?;
        root.assign_ids(&mut 1);

        Ok(Self
        {
            root,
            variable_order: (0..variable_count).collect(),
            level_widths: vec![1; variable_count],
            node_count,
            method: None,
            hybrid_size_factor: 2.0,
            adjacent_variable_exchanges: 0,
            block_exchanges: 0,
        })
    }

    pub fn with_variable_count(
        variable_count: usize,
        node_count: usize,
    ) -> Result<Self, BddReorderError>
    {
        Self::new(VariableBlock::balanced(variable_count)?, node_count)
    }

    pub fn root(&self) -> &VariableBlock
    {
        &self.root
    }

    pub fn root_mut(&mut self) -> &mut VariableBlock
    {
        &mut self.root
    }

    pub fn variable_order(&self) -> &[usize]
    {
        &self.variable_order
    }

    pub fn node_count(&self) -> usize
    {
        self.node_count
    }

    pub fn hybrid_size_factor(&self) -> f64
    {
        self.hybrid_size_factor
    }

    pub fn stats(&self) -> BddReorderStats
    {
        BddReorderStats
        {
            node_count: self.node_count,
            adjacent_variable_exchanges: self.adjacent_variable_exchanges,
            block_exchanges: self.block_exchanges,
        }
    }

    pub fn set_level_widths(&mut self, widths: Vec<usize>) -> Result<(), BddReorderError>
    {
        if widths.len() != self.variable_order.len()
        {
            return Err(BddReorderError::LevelWidthMismatch
            {
                expected: self.variable_order.len(),
                actual: widths.len(),
            });
        }

        self.level_widths = widths;
        Ok(())
    }

    pub fn set_dynamic_reordering(&mut self, method: Option<DynamicReorderMethod>)
    {
        self.method = method;
        self.hybrid_size_factor = if method == Some(DynamicReorderMethod::Hybrid)
        {
            2.0
        }
        else
        {
            0.0
        };
    }

    pub fn set_block_reorderable(
        &mut self,
        child_path: &[usize],
        reorderable: bool,
    ) -> Result<(), BddReorderError>
    {
        let block = block_at_path_mut(&mut self.root, child_path)?;
        block.set_reorderable(reorderable);
        Ok(())
    }

    pub fn reorder<F>(&mut self, mut cost: F) -> Result<bool, BddReorderError>
    where
        F: FnMut(&[usize]) -> usize,
    {
        match self.method
        {
            Some(DynamicReorderMethod::StableWindow3) => self.reorder_stable_window3(&mut cost),
            Some(DynamicReorderMethod::Sift) => self.reorder_sift(2.0, &mut cost),
            Some(DynamicReorderMethod::Hybrid) => self.reorder_hybrid(None, &mut cost),
            None => Ok(false),
        }
    }

    pub fn reorder_stable_window3<F>(&mut self, mut cost: F) -> Result<bool, BddReorderError>
    where
        F: FnMut(&[usize]) -> usize,
    {
        stable_window3_aux(
            &mut self.root,
            &mut self.variable_order,
            &mut self.node_count,
            &mut self.adjacent_variable_exchanges,
            &mut self.block_exchanges,
            &mut cost,
        )
    }

    pub fn reorder_sift<F>(
        &mut self,
        max_size_factor: f64,
        mut cost: F,
    ) -> Result<bool, BddReorderError>
    where
        F: FnMut(&[usize]) -> usize,
    {
        sift_aux(
            &mut self.root,
            &self.level_widths,
            &mut self.variable_order,
            &mut self.node_count,
            &mut self.adjacent_variable_exchanges,
            &mut self.block_exchanges,
            max_size_factor,
            &mut cost,
        )
    }

    pub fn reorder_hybrid<F>(
        &mut self,
        node_limit: Option<usize>,
        mut cost: F,
    ) -> Result<bool, BddReorderError>
    where
        F: FnMut(&[usize]) -> usize,
    {
        let start_nodes = self.node_count;
        let mut max_size_factor = self.hybrid_size_factor;

        if max_size_factor > 2.0 || start_nodes < 10_000
        {
            max_size_factor = 2.0;
        }

        let moved = sift_aux(
            &mut self.root,
            &self.level_widths,
            &mut self.variable_order,
            &mut self.node_count,
            &mut self.adjacent_variable_exchanges,
            &mut self.block_exchanges,
            max_size_factor_for_limit(max_size_factor, start_nodes, node_limit),
            &mut cost,
        )?;

        self.hybrid_size_factor = if start_nodes == 0
        {
            2.0
        }
        else
        {
            1.0 + (2.0 * start_nodes.saturating_sub(self.node_count) as f64)
                / start_nodes as f64
        };

        Ok(moved)
    }
}

fn max_size_factor_for_limit(
    factor: f64,
    start_nodes: usize,
    node_limit: Option<usize>,
) -> f64
{
    let Some(node_limit) = node_limit
    else
    {
        return factor;
    };

    if start_nodes == 0
    {
        return factor;
    }

    factor.min(node_limit as f64 / start_nodes as f64)
}

fn block_at_path_mut<'a>(
    mut block: &'a mut VariableBlock,
    path: &[usize],
) -> Result<&'a mut VariableBlock, BddReorderError>
{
    for &child_index in path
    {
        let child_count = block.children.len();
        block = block
            .children
            .get_mut(child_index)
            .ok_or(BddReorderError::InvalidChildIndex
            {
                child_index,
                child_count,
            })?;
    }

    Ok(block)
}

fn stable_window3_aux<F>(
    block: &mut VariableBlock,
    variable_order: &mut [usize],
    node_count: &mut usize,
    adjacent_variable_exchanges: &mut usize,
    block_exchanges: &mut usize,
    cost: &mut F,
) -> Result<bool, BddReorderError>
where
    F: FnMut(&[usize]) -> usize,
{
    let mut moved_any = false;

    if block.reorderable && block.children.len() > 1
    {
        let mut levels = vec![true; block.children.len().saturating_sub(1)];

        loop
        {
            let mut any_swapped = false;

            for index in 0..block.children.len() - 1
            {
                if levels[index]
                {
                    let moved = if index < block.children.len() - 2
                    {
                        reorder_window3(
                            block,
                            index,
                            variable_order,
                            node_count,
                            adjacent_variable_exchanges,
                            block_exchanges,
                            cost,
                        )?
                    }
                    else
                    {
                        reorder_window2(
                            block,
                            index,
                            variable_order,
                            node_count,
                            adjacent_variable_exchanges,
                            block_exchanges,
                            cost,
                        )?
                    };

                    if moved
                    {
                        mark_changed_window(&mut levels, index);
                        any_swapped = true;
                        moved_any = true;
                    }
                    else
                    {
                        levels[index] = false;
                    }
                }
            }

            if !any_swapped
            {
                break;
            }
        }
    }

    for child in &mut block.children
    {
        moved_any |= stable_window3_aux(
            child,
            variable_order,
            node_count,
            adjacent_variable_exchanges,
            block_exchanges,
            cost,
        )?;
    }

    Ok(moved_any)
}

fn mark_changed_window(levels: &mut [bool], index: usize)
{
    let start = index.saturating_sub(2);
    let end = (index + 4).min(levels.len().saturating_sub(1));

    for level in &mut levels[start..=end]
    {
        *level = true;
    }
}

fn reorder_window2<F>(
    block: &mut VariableBlock,
    index: usize,
    variable_order: &mut [usize],
    node_count: &mut usize,
    adjacent_variable_exchanges: &mut usize,
    block_exchanges: &mut usize,
    cost: &mut F,
) -> Result<bool, BddReorderError>
where
    F: FnMut(&[usize]) -> usize,
{
    let best_size = *node_count;

    exchange_var_blocks(
        block,
        index,
        variable_order,
        node_count,
        adjacent_variable_exchanges,
        block_exchanges,
        cost,
    )?;

    if *node_count < best_size
    {
        return Ok(true);
    }

    exchange_var_blocks(
        block,
        index,
        variable_order,
        node_count,
        adjacent_variable_exchanges,
        block_exchanges,
        cost,
    )?;

    Ok(false)
}

fn reorder_window3<F>(
    block: &mut VariableBlock,
    index: usize,
    variable_order: &mut [usize],
    node_count: &mut usize,
    adjacent_variable_exchanges: &mut usize,
    block_exchanges: &mut usize,
    cost: &mut F,
) -> Result<bool, BddReorderError>
where
    F: FnMut(&[usize]) -> usize,
{
    let mut best = 0;
    let mut best_size = *node_count;

    exchange_var_blocks(
        block,
        index,
        variable_order,
        node_count,
        adjacent_variable_exchanges,
        block_exchanges,
        cost,
    )?;
    update_best(*node_count, &mut best_size, &mut best, 1);

    exchange_var_blocks(
        block,
        index + 1,
        variable_order,
        node_count,
        adjacent_variable_exchanges,
        block_exchanges,
        cost,
    )?;
    update_best(*node_count, &mut best_size, &mut best, 2);

    exchange_var_blocks(
        block,
        index,
        variable_order,
        node_count,
        adjacent_variable_exchanges,
        block_exchanges,
        cost,
    )?;
    update_best(*node_count, &mut best_size, &mut best, 3);

    exchange_var_blocks(
        block,
        index + 1,
        variable_order,
        node_count,
        adjacent_variable_exchanges,
        block_exchanges,
        cost,
    )?;
    update_best(*node_count, &mut best_size, &mut best, 4);

    exchange_var_blocks(
        block,
        index,
        variable_order,
        node_count,
        adjacent_variable_exchanges,
        block_exchanges,
        cost,
    )?;
    update_best(*node_count, &mut best_size, &mut best, 5);

    match best
    {
        0 =>
        {
            exchange_var_blocks(
                block,
                index + 1,
                variable_order,
                node_count,
                adjacent_variable_exchanges,
                block_exchanges,
                cost,
            )?;
        }
        1 =>
        {
            exchange_var_blocks(
                block,
                index + 1,
                variable_order,
                node_count,
                adjacent_variable_exchanges,
                block_exchanges,
                cost,
            )?;
            exchange_var_blocks(
                block,
                index,
                variable_order,
                node_count,
                adjacent_variable_exchanges,
                block_exchanges,
                cost,
            )?;
        }
        2 =>
        {
            exchange_var_blocks(
                block,
                index + 1,
                variable_order,
                node_count,
                adjacent_variable_exchanges,
                block_exchanges,
                cost,
            )?;
            exchange_var_blocks(
                block,
                index,
                variable_order,
                node_count,
                adjacent_variable_exchanges,
                block_exchanges,
                cost,
            )?;
            exchange_var_blocks(
                block,
                index + 1,
                variable_order,
                node_count,
                adjacent_variable_exchanges,
                block_exchanges,
                cost,
            )?;
        }
        3 =>
        {
            exchange_var_blocks(
                block,
                index,
                variable_order,
                node_count,
                adjacent_variable_exchanges,
                block_exchanges,
                cost,
            )?;
            exchange_var_blocks(
                block,
                index + 1,
                variable_order,
                node_count,
                adjacent_variable_exchanges,
                block_exchanges,
                cost,
            )?;
        }
        4 =>
        {
            exchange_var_blocks(
                block,
                index,
                variable_order,
                node_count,
                adjacent_variable_exchanges,
                block_exchanges,
                cost,
            )?;
        }
        5 =>
        {
        }
        _ => unreachable!(),
    }

    Ok(best > 0)
}

fn update_best(size: usize, best_size: &mut usize, best: &mut usize, candidate: usize)
{
    if size < *best_size
    {
        *best_size = size;
        *best = candidate;
    }
}

fn sift_aux<F>(
    block: &mut VariableBlock,
    level_widths: &[usize],
    variable_order: &mut [usize],
    node_count: &mut usize,
    adjacent_variable_exchanges: &mut usize,
    block_exchanges: &mut usize,
    max_size_factor: f64,
    cost: &mut F,
) -> Result<bool, BddReorderError>
where
    F: FnMut(&[usize]) -> usize,
{
    let mut moved_any = false;

    if block.reorderable
    {
        let mut to_sift: Vec<usize> = block.children.iter().map(|child| child.id).collect();

        while let Some((widest_position, max_width)) = widest_remaining(block, &to_sift, level_widths)
        {
            if max_width <= 1
            {
                break;
            }

            let child_id = to_sift.remove(widest_position);
            let child_position = block
                .children
                .iter()
                .position(|child| child.id == child_id)
                .expect("to_sift only contains child block ids");

            moved_any |= sift_block(
                block,
                child_position,
                variable_order,
                node_count,
                adjacent_variable_exchanges,
                block_exchanges,
                max_size_factor,
                cost,
            )?;
        }
    }

    for child in &mut block.children
    {
        moved_any |= sift_aux(
            child,
            level_widths,
            variable_order,
            node_count,
            adjacent_variable_exchanges,
            block_exchanges,
            max_size_factor,
            cost,
        )?;
    }

    Ok(moved_any)
}

fn widest_remaining(
    block: &VariableBlock,
    to_sift: &[usize],
    level_widths: &[usize],
) -> Option<(usize, usize)>
{
    to_sift
        .iter()
        .enumerate()
        .filter_map(|(position, id)| {
            let child = block.children.iter().find(|child| child.id == *id)?;
            let width_sum: usize = (child.first_index..=child.last_index)
                .map(|index| level_widths[index])
                .sum();
            let width = width_sum / child.variable_count();
            Some((position, width))
        })
        .max_by_key(|(_, width)| *width)
}

fn sift_block<F>(
    block: &mut VariableBlock,
    start_pos: usize,
    variable_order: &mut [usize],
    node_count: &mut usize,
    adjacent_variable_exchanges: &mut usize,
    block_exchanges: &mut usize,
    max_size_factor: f64,
    cost: &mut F,
) -> Result<bool, BddReorderError>
where
    F: FnMut(&[usize]) -> usize,
{
    let start_size = *node_count;
    let mut best_size = start_size;
    let mut best_pos = start_pos;
    let mut curr_size = start_size;
    let mut curr_pos = start_pos;
    let max_size = (max_size_factor * start_size as f64) as usize;

    while curr_pos < block.children.len() - 1 && curr_size <= max_size
    {
        exchange_var_blocks(
            block,
            curr_pos,
            variable_order,
            node_count,
            adjacent_variable_exchanges,
            block_exchanges,
            cost,
        )?;
        curr_pos += 1;
        curr_size = *node_count;

        if curr_size < best_size
        {
            best_size = curr_size;
            best_pos = curr_pos;
        }
    }

    while curr_pos != start_pos
    {
        curr_pos -= 1;
        exchange_var_blocks(
            block,
            curr_pos,
            variable_order,
            node_count,
            adjacent_variable_exchanges,
            block_exchanges,
            cost,
        )?;
    }

    curr_size = start_size;

    while curr_pos > 0 && curr_size <= max_size
    {
        curr_pos -= 1;
        exchange_var_blocks(
            block,
            curr_pos,
            variable_order,
            node_count,
            adjacent_variable_exchanges,
            block_exchanges,
            cost,
        )?;
        curr_size = *node_count;

        if curr_size < best_size
        {
            best_size = curr_size;
            best_pos = curr_pos;
        }
    }

    while curr_pos != best_pos
    {
        exchange_var_blocks(
            block,
            curr_pos,
            variable_order,
            node_count,
            adjacent_variable_exchanges,
            block_exchanges,
            cost,
        )?;
        curr_pos += 1;
    }

    Ok(best_pos != start_pos)
}

fn exchange_var_blocks<F>(
    parent: &mut VariableBlock,
    child_index: usize,
    variable_order: &mut [usize],
    node_count: &mut usize,
    adjacent_variable_exchanges: &mut usize,
    block_exchanges: &mut usize,
    cost: &mut F,
) -> Result<(), BddReorderError>
where
    F: FnMut(&[usize]) -> usize,
{
    if child_index + 1 >= parent.children.len()
    {
        return Err(BddReorderError::InvalidChildIndex
        {
            child_index,
            child_count: parent.children.len(),
        });
    }

    let left_width = parent.children[child_index].variable_count();
    let right_width = parent.children[child_index + 1].variable_count();
    let right_first = parent.children[child_index + 1].first_index;

    for step in 0..left_width + right_width - 1
    {
        let start = step.saturating_sub(left_width - 1);
        let end = step.min(right_width - 1);

        for offset in start..=end
        {
            let right_index = right_first + offset - step + offset;
            exchange_adjacent_variables(
                variable_order,
                right_index - 1,
                adjacent_variable_exchanges,
            )?;
        }
    }

    parent.children[child_index].shift_indices(right_width as isize);
    parent.children[child_index + 1].shift_indices(-(left_width as isize));
    parent.children.swap(child_index, child_index + 1);
    *block_exchanges += 1;
    *node_count = cost(variable_order);

    Ok(())
}

fn exchange_adjacent_variables(
    variable_order: &mut [usize],
    left_index: usize,
    adjacent_variable_exchanges: &mut usize,
) -> Result<(), BddReorderError>
{
    let right_index = left_index + 1;

    if right_index >= variable_order.len()
    {
        return Err(BddReorderError::MissingAdjacentVariable
        {
            left_index,
            right_index,
        });
    }

    variable_order.swap(left_index, right_index);
    *adjacent_variable_exchanges += 1;

    Ok(())
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn order_cost(order: &[usize], target: &[usize], base: usize) -> usize
    {
        base + order
            .iter()
            .zip(target)
            .filter(|(actual, expected)| actual != expected)
            .count()
    }

    #[test]
    fn stable_window2_keeps_an_improving_adjacent_swap()
    {
        let mut manager = BddReorderManager::with_variable_count(2, 12).unwrap();

        let moved = manager
            .reorder_stable_window3(|order| order_cost(order, &[1, 0], 10))
            .unwrap();

        assert!(moved);
        assert_eq!(manager.variable_order(), &[1, 0]);
        assert_eq!(manager.node_count(), 10);
    }

    #[test]
    fn stable_window2_reverts_non_improving_swaps()
    {
        let mut manager = BddReorderManager::with_variable_count(2, 10).unwrap();

        let moved = manager
            .reorder_stable_window3(|order| order_cost(order, &[0, 1], 10))
            .unwrap();

        assert!(!moved);
        assert_eq!(manager.variable_order(), &[0, 1]);
        assert_eq!(manager.node_count(), 10);
    }

    #[test]
    fn stable_window3_selects_best_permutation_from_three_blocks()
    {
        let mut manager = BddReorderManager::with_variable_count(3, 13).unwrap();

        let moved = manager
            .reorder_stable_window3(|order| order_cost(order, &[2, 0, 1], 10))
            .unwrap();

        assert!(moved);
        assert_eq!(manager.variable_order(), &[2, 0, 1]);
        assert_eq!(manager.node_count(), 10);
    }

    #[test]
    fn non_reorderable_block_is_left_unchanged()
    {
        let mut manager = BddReorderManager::with_variable_count(3, 13).unwrap();
        manager.root_mut().set_reorderable(false);

        let moved = manager
            .reorder_stable_window3(|order| order_cost(order, &[2, 0, 1], 10))
            .unwrap();

        assert!(!moved);
        assert_eq!(manager.variable_order(), &[0, 1, 2]);
        assert_eq!(manager.node_count(), 13);
    }

    #[test]
    fn nested_blocks_slide_past_each_other_by_interleaving_variables()
    {
        let left = VariableBlock::new(0, 1, false, vec![VariableBlock::leaf(0), VariableBlock::leaf(1)])
            .unwrap();
        let right = VariableBlock::new(2, 3, false, vec![VariableBlock::leaf(2), VariableBlock::leaf(3)])
            .unwrap();
        let root = VariableBlock::new(0, 3, true, vec![left, right]).unwrap();
        let mut manager = BddReorderManager::new(root, 14).unwrap();

        let moved = manager
            .reorder_stable_window3(|order| order_cost(order, &[2, 3, 0, 1], 10))
            .unwrap();

        assert!(moved);
        assert_eq!(manager.variable_order(), &[2, 3, 0, 1]);
        assert!(manager.stats().adjacent_variable_exchanges >= 4);
    }

    #[test]
    fn sift_moves_widest_block_to_best_position()
    {
        let mut manager = BddReorderManager::with_variable_count(4, 14).unwrap();
        manager.set_level_widths(vec![1, 8, 2, 1]).unwrap();

        let moved = manager
            .reorder_sift(2.0, |order| order_cost(order, &[0, 2, 3, 1], 10))
            .unwrap();

        assert!(moved);
        assert_eq!(manager.variable_order(), &[0, 2, 3, 1]);
        assert_eq!(manager.node_count(), 10);
    }

    #[test]
    fn dynamic_hybrid_uses_sift_and_updates_factor_from_node_reduction()
    {
        let mut manager = BddReorderManager::with_variable_count(3, 12_000).unwrap();
        manager.set_level_widths(vec![9, 1, 1]).unwrap();
        manager.set_dynamic_reordering(Some(DynamicReorderMethod::Hybrid));

        let moved = manager
            .reorder(|order| order_cost(order, &[1, 2, 0], 9_000))
            .unwrap();

        assert!(moved);
        assert_eq!(manager.variable_order(), &[1, 2, 0]);
        assert_eq!(manager.node_count(), 9_000);
        assert_eq!(manager.hybrid_size_factor(), 1.5);
    }

    #[test]
    fn invalid_level_widths_are_rejected()
    {
        let mut manager = BddReorderManager::with_variable_count(3, 10).unwrap();

        let error = manager.set_level_widths(vec![1, 2]).unwrap_err();

        assert_eq!(
            error,
            BddReorderError::LevelWidthMismatch
            {
                expected: 3,
                actual: 2
            }
        );
    }

    #[test]
    fn source_has_no_legacy_c_abi_or_dependency_metadata()
    {
        let source = include_str!("bddreorder.rs");

        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains(concat!("pub ", "extern")));
        assert!(!source.contains(concat!("extern ", "\"", "C", "\"")));
        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("Logic", "Friday1", "-", "8j8")));
    }
}
