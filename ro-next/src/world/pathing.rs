//! Implements pathfinding algorithms.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use hashbrown::{HashMap, HashSet};
use ragnarok_packets::{AttackRange, TilePosition};

const MOVE_DIAGONAL_COST: usize = 14;
const MOVE_ORTHOGONAL_COST: usize = 10;
/// The maximum size a walkable path can have (the client's movement packet
/// limit for entity walking).
pub const MAX_WALK_PATH_SIZE: usize = 32;
/// Maximum route length for the navigation guide. A route can be much longer
/// than a single movement step; the limit only guards against a runaway A*
/// search on a malformed map.
pub const MAX_NAVIGATION_PATH_SIZE: usize = 2048;
/// Maximum number of nodes the navigation A* search may expand before it gives
/// up. This keeps `/navi` responsive even when the destination is unreachable.
const MAX_NAVIGATION_EXPANSIONS: usize = 400_000;

/// Essential trait that is needed to be implements for pathfinding.
pub trait Traversable {
    /// Must return `true` if the position can be walked on.
    fn is_walkable(&self, position: TilePosition) -> bool;
}

#[derive(Copy, Clone, Eq, PartialEq)]
struct PathNode {
    position: TilePosition,
    f_score: usize,
    g_score: usize,
}

impl Ord for PathNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f_score.cmp(&self.f_score)
    }
}

impl PartialOrd for PathNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Pathfinding algorithm for entity map navigation.
#[derive(Default)]
pub struct PathFinder {
    open_set: BinaryHeap<PathNode>,
    closed_set: HashSet<TilePosition>,
    came_from: HashMap<TilePosition, TilePosition>,
    g_scores: HashMap<TilePosition, usize>,
    path: Vec<TilePosition>,
    neighbors: Vec<TilePosition>,
}

impl PathFinder {
    /// Returns the shortest walkable path between start and goal. Uses a simple
    /// A* search algorithm like the legacy client and alternative server
    /// implementations. It must have the same behavior, or else we would
    /// "desync" with our client movement prediction.
    pub fn find_walkable_path(&mut self, map: &impl Traversable, start: TilePosition, goal: TilePosition) -> Option<&[TilePosition]> {
        self.find_walkable_path_in_range(map, start, goal, AttackRange(0))
    }

    /// Returns the shortest walkable path between start and one attack range
    /// away from the goal. Uses a simple A* search algorithm like the legacy
    /// client and alternative server implementations. It must have the same
    /// behavior, or else we would "desync" with our client movement prediction.
    pub fn find_walkable_path_in_range(
        &mut self,
        map: &impl Traversable,
        start: TilePosition,
        goal: TilePosition,
        attack_range: AttackRange,
    ) -> Option<&[TilePosition]> {
        self.find_path_in_range(map, start, goal, attack_range, MAX_WALK_PATH_SIZE, usize::MAX)
    }

    /// Finds the full walkable route between start and goal for the navigation
    /// guide. Unlike [`Self::find_walkable_path_in_range`] the route is not
    /// capped at the 32-tile movement packet limit, so it can span a whole map.
    ///
    /// Returns [`None`] when no route exists, the route is longer than
    /// [`MAX_NAVIGATION_PATH_SIZE`] or the search exceeded its node budget.
    pub fn find_navigation_path(&mut self, map: &impl Traversable, start: TilePosition, goal: TilePosition) -> Option<&[TilePosition]> {
        self.find_path_in_range(
            map,
            start,
            goal,
            AttackRange(0),
            MAX_NAVIGATION_PATH_SIZE,
            MAX_NAVIGATION_EXPANSIONS,
        )
    }

    fn find_path_in_range(
        &mut self,
        map: &impl Traversable,
        start: TilePosition,
        goal: TilePosition,
        attack_range: AttackRange,
        max_path_size: usize,
        max_expansions: usize,
    ) -> Option<&[TilePosition]> {
        self.open_set.clear();
        self.closed_set.clear();
        self.came_from.clear();
        self.g_scores.clear();
        self.path.clear();

        self.open_set.push(PathNode {
            position: start,
            g_score: 0,
            f_score: Self::heuristic(start, goal),
        });
        self.g_scores.insert(start, 0);

        let mut expansions = 0;

        while let Some(current) = self.open_set.pop() {
            if current.position.x.abs_diff(goal.x).max(current.position.y.abs_diff(goal.y)) <= attack_range.0 {
                return match self.reconstruct_path(start, current.position, max_path_size) {
                    true => Some(&self.path),
                    false => None,
                };
            }

            expansions += 1;
            if expansions > max_expansions {
                return None;
            }

            if self.closed_set.contains(&current.position) {
                continue;
            }
            self.closed_set.insert(current.position);

            self.find_neighbors(map, current.position);

            for neighbor in self.neighbors.drain(..) {
                if self.closed_set.contains(&neighbor) {
                    continue;
                }

                let movement_cost = if neighbor.x != current.position.x && neighbor.y != current.position.y {
                    MOVE_DIAGONAL_COST
                } else {
                    MOVE_ORTHOGONAL_COST
                };

                let tentative_g_score = current.g_score + movement_cost;

                if tentative_g_score < self.g_scores.get(&neighbor).copied().unwrap_or(usize::MAX) {
                    self.came_from.insert(neighbor, current.position);
                    self.g_scores.insert(neighbor, tentative_g_score);

                    let h_score = Self::heuristic(neighbor, goal);
                    let f_score = tentative_g_score + h_score;

                    self.open_set.push(PathNode {
                        position: neighbor,
                        g_score: tentative_g_score,
                        f_score,
                    });
                }
            }
        }

        None
    }

    fn heuristic(start: TilePosition, goal: TilePosition) -> usize {
        let dx = (start.x as isize - goal.x as isize).unsigned_abs();
        let dy = (start.y as isize - goal.y as isize).unsigned_abs();
        let manhattan_distance = dx + dy;
        MOVE_ORTHOGONAL_COST * manhattan_distance
    }

    fn find_neighbors(&mut self, map: &impl Traversable, position: TilePosition) {
        let orthogonal_neighbors = [(1, 0), (-1, 0), (0, 1), (0, -1)];

        for (dx, dy) in orthogonal_neighbors {
            let new_x = position.x.wrapping_add_signed(dx);
            let new_y = position.y.wrapping_add_signed(dy);
            let new_position = TilePosition { x: new_x, y: new_y };

            if map.is_walkable(new_position) {
                self.neighbors.push(new_position);
            }
        }

        let diagonal_neighbors = [(1, 1), (1, -1), (-1, 1), (-1, -1)];

        for (dx, dy) in diagonal_neighbors {
            let new_x = position.x.wrapping_add_signed(dx);
            let new_y = position.y.wrapping_add_signed(dy);
            let new_position = TilePosition { x: new_x, y: new_y };

            // Only allow diagonal neighbors when both adjacent orthogonal neighbors are
            // also walkable.
            if map.is_walkable(new_position)
                && map.is_walkable(TilePosition { x: position.x, y: new_y })
                && map.is_walkable(TilePosition { x: new_x, y: position.y })
            {
                self.neighbors.push(new_position);
            }
        }
    }

    fn reconstruct_path(&mut self, start: TilePosition, goal: TilePosition, max_path_size: usize) -> bool {
        let mut current = goal;

        while current != start {
            self.path.push(current);
            current = *self.came_from.get(&current).unwrap();

            if self.path.len() >= max_path_size {
                return false;
            }
        }

        self.path.push(start);
        self.path.reverse();

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestMap {
        width: u16,
        height: u16,
        not_walkable: HashSet<TilePosition>,
    }

    impl TestMap {
        fn new(width: u16, height: u16) -> Self {
            Self {
                width,
                height,
                not_walkable: HashSet::new(),
            }
        }

        fn set_unwalkable(&mut self, points: &[TilePosition]) {
            for point in points {
                self.not_walkable.insert(*point);
            }
        }
    }

    impl Traversable for TestMap {
        fn is_walkable(&self, position: TilePosition) -> bool {
            position.x < self.width && position.y < self.height && !self.not_walkable.contains(&position)
        }
    }

    #[test]
    fn test_straight_path() {
        let map = TestMap::new(10, 10);
        let mut pathfinder = PathFinder::default();

        let start = TilePosition { x: 0, y: 0 };
        let goal = TilePosition { x: 3, y: 0 };

        let path = pathfinder.find_walkable_path(&map, start, goal).unwrap();
        assert_eq!(path, vec![
            TilePosition { x: 0, y: 0 },
            TilePosition { x: 1, y: 0 },
            TilePosition { x: 2, y: 0 },
            TilePosition { x: 3, y: 0 },
        ]);
    }

    #[test]
    fn test_diagonal_path() {
        let map = TestMap::new(10, 10);
        let mut pathfinder = PathFinder::default();

        let start = TilePosition { x: 0, y: 0 };
        let goal = TilePosition { x: 3, y: 3 };

        let path = pathfinder.find_walkable_path(&map, start, goal).unwrap();
        assert_eq!(path, vec![
            TilePosition { x: 0, y: 0 },
            TilePosition { x: 1, y: 1 },
            TilePosition { x: 2, y: 2 },
            TilePosition { x: 3, y: 3 },
        ]);
    }

    #[test]
    fn test_path_with_obstacle() {
        let mut map = TestMap::new(5, 5);
        map.set_unwalkable(&[TilePosition { x: 1, y: 1 }, TilePosition { x: 1, y: 2 }, TilePosition {
            x: 1,
            y: 3,
        }]);

        let mut pathfinder = PathFinder::default();
        let start = TilePosition { x: 0, y: 0 };
        let goal = TilePosition { x: 2, y: 2 };

        let path = pathfinder.find_walkable_path(&map, start, goal).unwrap();

        assert_eq!(path, vec![
            TilePosition { x: 0, y: 0 },
            TilePosition { x: 1, y: 0 },
            TilePosition { x: 2, y: 0 },
            TilePosition { x: 2, y: 1 },
            TilePosition { x: 2, y: 2 },
        ]);
    }

    #[test]
    fn navigation_paths_are_not_capped_at_the_movement_limit() {
        let map = TestMap::new(50, 5);
        let mut pathfinder = PathFinder::default();

        let start = TilePosition { x: 0, y: 0 };
        let goal = TilePosition { x: 40, y: 0 };

        // The movement path finder keeps the 32-tile packet limit.
        assert!(pathfinder.find_walkable_path(&map, start, goal).is_none());

        // The navigation route may span the whole map.
        let path = pathfinder.find_navigation_path(&map, start, goal).unwrap();
        assert_eq!(path.len(), 41);
        assert_eq!(path.first(), Some(&start));
        assert_eq!(path.last(), Some(&goal));
    }

    #[test]
    fn test_no_path_possible() {
        let mut map = TestMap::new(5, 5);

        map.set_unwalkable(&[
            TilePosition { x: 1, y: 0 },
            TilePosition { x: 1, y: 1 },
            TilePosition { x: 1, y: 2 },
            TilePosition { x: 1, y: 3 },
            TilePosition { x: 1, y: 4 },
        ]);

        let mut pathfinder = PathFinder::default();

        let start = TilePosition { x: 0, y: 2 };
        let goal = TilePosition { x: 2, y: 2 };

        assert!(pathfinder.find_walkable_path(&map, start, goal).is_none());
    }
}
