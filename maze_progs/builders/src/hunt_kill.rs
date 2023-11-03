use std::io;

use crate::build;
use crossterm::{
    execute, queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
};
use maze;
use rand::{seq::SliceRandom, thread_rng, Rng};
use speed;
use std::{thread, time};

const CARVER: char = '█';
// Same color for each looks good for now change for style.
const CARVING: u8 = 9;
const HUNTING: u8 = 9;
const WALL_UP_DOWN_RIGHT: usize = 0b0111;
const WALL_UP_DOWN_LEFT: usize = 0b1101;
const WALL_LEFT_RIGHT: usize = 0b1010;

pub fn generate_maze(monitor: monitor::SolverReceiver) {
    let mut lk = match monitor.solver.lock() {
        Ok(l) => l,
        Err(_) => print::maze_panic!("uncontested lock failure"),
    };
    build::fill_maze_with_walls(&mut lk.maze);
    let mut gen = thread_rng();
    let start: maze::Point = maze::Point {
        row: 2 * (gen.gen_range(1..lk.maze.row_size() - 2) / 2) + 1,
        col: 2 * (gen.gen_range(1..lk.maze.col_size() - 2) / 2) + 1,
    };
    let mut random_direction_indices: Vec<usize> = (0..build::NUM_DIRECTIONS).collect();
    let mut cur: maze::Point = start;
    let mut highest_completed_row = 1;
    'carving: loop {
        random_direction_indices.shuffle(&mut gen);
        for &i in random_direction_indices.iter() {
            let direction = &build::GENERATE_DIRECTIONS[i];
            let branch = maze::Point {
                row: cur.row + direction.row,
                col: cur.col + direction.col,
            };
            if build::can_build_new_square(&lk.maze, branch) {
                build::join_squares(&mut lk.maze, cur, branch);
                cur = branch;
                continue 'carving;
            }
        }

        let mut set_highest_completed_row = false;
        for r in (highest_completed_row..lk.maze.row_size() - 1).step_by(2) {
            for c in (1..lk.maze.col_size() - 1).step_by(2) {
                let start_candidate = maze::Point { row: r, col: c };
                if (lk.maze.get(r, c) & build::BUILDER_BIT) == 0 {
                    if !set_highest_completed_row {
                        highest_completed_row = r;
                        set_highest_completed_row = true;
                    }
                    for dir in &build::GENERATE_DIRECTIONS {
                        let next = maze::Point {
                            row: r + dir.row,
                            col: c + dir.col,
                        };
                        if build::is_square_within_perimeter_walls(&lk.maze, next)
                            && (lk.maze.get(next.row, next.col) & build::BUILDER_BIT) != 0
                        {
                            build::join_squares(&mut lk.maze, start_candidate, next);
                            cur = start_candidate;
                            continue 'carving;
                        }
                    }
                }
            }
        }
        return;
    }
}

pub fn animate_maze(monitor: monitor::SolverReceiver, speed: speed::Speed) {
    let mut lk = match monitor.solver.lock() {
        Ok(l) => l,
        Err(_) => print::maze_panic!("uncontested lock failure"),
    };
    if lk.maze.is_mini() {
        drop(lk);
        animate_mini_maze(monitor, speed);
        return;
    }
    let animation: build::SpeedUnit = build::BUILDER_SPEEDS[speed as usize];
    build::fill_maze_with_walls(&mut lk.maze);
    build::flush_grid(&lk.maze);
    build::print_overlap_key_animated(&lk.maze);
    let mut gen = thread_rng();
    let start: maze::Point = maze::Point {
        row: 2 * (gen.gen_range(1..lk.maze.row_size() - 2) / 2) + 1,
        col: 2 * (gen.gen_range(1..lk.maze.col_size() - 2) / 2) + 1,
    };
    let mut random_direction_indices: Vec<usize> = (0..build::NUM_DIRECTIONS).collect();
    let mut cur: maze::Point = start;
    let mut highest_completed_row = 1;
    'carving: loop {
        if monitor.exit() {
            return;
        }
        print::set_cursor_position(cur, lk.maze.offset());
        execute!(
            io::stdout(),
            SetForegroundColor(Color::AnsiValue(CARVING)),
            Print(CARVER),
            ResetColor
        )
        .expect("Printer broke");
        thread::sleep(time::Duration::from_micros(animation));
        random_direction_indices.shuffle(&mut gen);
        for &i in random_direction_indices.iter() {
            let direction = &build::GENERATE_DIRECTIONS[i];
            let branch = maze::Point {
                row: cur.row + direction.row,
                col: cur.col + direction.col,
            };
            if build::can_build_new_square(&lk.maze, branch) {
                build::join_squares_animated(&mut lk.maze, cur, branch, animation);
                print::set_cursor_position(branch, lk.maze.offset());
                execute!(
                    io::stdout(),
                    SetForegroundColor(Color::AnsiValue(CARVING)),
                    Print(CARVER),
                    ResetColor
                )
                .expect("Printer broke");
                thread::sleep(time::Duration::from_micros(animation));
                cur = branch;
                continue 'carving;
            }
        }

        let mut set_highest_completed_row = false;
        for r in (highest_completed_row..lk.maze.row_size() - 1).step_by(2) {
            shoot_hunter_laser(&lk.maze, r);
            for c in (1..lk.maze.col_size() - 1).step_by(2) {
                let start_candidate = maze::Point { row: r, col: c };
                if (lk.maze.get(r, c) & build::BUILDER_BIT) == 0 {
                    if !set_highest_completed_row {
                        highest_completed_row = r;
                        set_highest_completed_row = true;
                    }
                    for dir in &build::GENERATE_DIRECTIONS {
                        let next = maze::Point {
                            row: r + dir.row,
                            col: c + dir.col,
                        };
                        if build::is_square_within_perimeter_walls(&lk.maze, next)
                            && (lk.maze.get(next.row, next.col) & build::BUILDER_BIT) != 0
                        {
                            build::join_squares_animated(
                                &mut lk.maze,
                                start_candidate,
                                next,
                                animation,
                            );
                            cur = start_candidate;
                            continue 'carving;
                        }
                    }
                }
            }
            reset_row(&lk.maze, r);
        }
        return;
    }
}

pub fn animate_mini_maze(monitor: monitor::SolverReceiver, speed: speed::Speed) {
    let mut lk = match monitor.solver.lock() {
        Ok(l) => l,
        Err(_) => print::maze_panic!("uncontested lock failure"),
    };
    let animation: build::SpeedUnit = build::BUILDER_SPEEDS[speed as usize];
    build::fill_maze_with_walls(&mut lk.maze);
    build::flush_grid(&lk.maze);
    build::print_overlap_key_animated(&lk.maze);
    let mut gen = thread_rng();
    let start: maze::Point = maze::Point {
        row: 2 * (gen.gen_range(1..lk.maze.row_size() - 2) / 2) + 1,
        col: 2 * (gen.gen_range(1..lk.maze.col_size() - 2) / 2) + 1,
    };
    let mut random_direction_indices: Vec<usize> = (0..build::NUM_DIRECTIONS).collect();
    let mut cur: maze::Point = start;
    let mut highest_completed_row = 1;
    'carving: loop {
        if monitor.exit() {
            return;
        }
        flush_mini_carver(&lk.maze, cur, animation);
        random_direction_indices.shuffle(&mut gen);
        for &i in random_direction_indices.iter() {
            let direction = &build::GENERATE_DIRECTIONS[i];
            let branch = maze::Point {
                row: cur.row + direction.row,
                col: cur.col + direction.col,
            };
            if build::can_build_new_square(&lk.maze, branch) {
                build::join_minis_animated(&mut lk.maze, cur, branch, animation);
                print::set_cursor_position(
                    maze::Point {
                        row: branch.row / 2,
                        col: branch.col,
                    },
                    lk.maze.offset(),
                );
                flush_mini_carver(&lk.maze, branch, animation);
                cur = branch;
                continue 'carving;
            }
        }

        let mut set_highest_completed_row = false;
        for r in (highest_completed_row..lk.maze.row_size() - 1).step_by(2) {
            shoot_mini_laser(&lk.maze, r);
            for c in (1..lk.maze.col_size() - 1).step_by(2) {
                let start_candidate = maze::Point { row: r, col: c };
                if (lk.maze.get(r, c) & build::BUILDER_BIT) == 0 {
                    if !set_highest_completed_row {
                        highest_completed_row = r;
                        set_highest_completed_row = true;
                    }
                    for dir in &build::GENERATE_DIRECTIONS {
                        let next = maze::Point {
                            row: r + dir.row,
                            col: c + dir.col,
                        };
                        if build::is_square_within_perimeter_walls(&lk.maze, next)
                            && (lk.maze.get(next.row, next.col) & build::BUILDER_BIT) != 0
                        {
                            build::join_minis_animated(
                                &mut lk.maze,
                                start_candidate,
                                next,
                                animation,
                            );
                            cur = start_candidate;
                            continue 'carving;
                        }
                    }
                }
            }
            reset_mini_row(&lk.maze, r);
        }
        return;
    }
}

fn shoot_hunter_laser(maze: &maze::Maze, current_row: i32) {
    let mut stdout = io::stdout();
    print::set_cursor_position(
        maze::Point {
            row: current_row,
            col: 0,
        },
        maze.offset(),
    );
    // Don't reset the color until the end.
    queue!(
        stdout,
        SetForegroundColor(Color::AnsiValue(HUNTING)),
        Print(maze.wall_char(WALL_UP_DOWN_RIGHT)),
    )
    .expect("Printer broke");
    for c in 1..maze.col_size() - 1 {
        print::set_cursor_position(
            maze::Point {
                row: current_row,
                col: c,
            },
            maze.offset(),
        );
        queue!(stdout, Print(maze.wall_char(WALL_LEFT_RIGHT)),).expect("Printer broke");
    }
    print::set_cursor_position(
        maze::Point {
            row: current_row,
            col: maze.col_size() - 1,
        },
        maze.offset(),
    );
    // Color is now reset.
    queue!(
        stdout,
        SetForegroundColor(Color::AnsiValue(HUNTING)),
        Print(maze.wall_char(WALL_UP_DOWN_LEFT)),
        ResetColor
    )
    .expect("Printer broke");
    print::flush();
}

fn shoot_mini_laser(maze: &maze::Maze, current_row: i32) {
    let mut stdout = io::stdout();
    for c in 0..maze.col_size() {
        print::set_cursor_position(
            maze::Point {
                row: current_row / 2,
                col: c,
            },
            maze.offset(),
        );
        if (maze.get(current_row + 1, c) & maze::PATH_BIT) == 0 {
            queue!(
                stdout,
                SetBackgroundColor(Color::AnsiValue(HUNTING)),
                Print('▄'),
                ResetColor
            )
            .expect("Printer broke");
        } else {
            queue!(
                stdout,
                SetBackgroundColor(Color::AnsiValue(HUNTING)),
                SetForegroundColor(Color::Black),
                Print('▄'),
                ResetColor
            )
            .expect("Printer broke");
        }
    }
    print::flush();
}

fn reset_row(maze: &maze::Maze, current_row: i32) {
    for c in 0..maze.col_size() {
        build::print_square(
            maze,
            maze::Point {
                row: current_row,
                col: c,
            },
        );
    }
    print::flush();
}

fn reset_mini_row(maze: &maze::Maze, current_row: i32) {
    for c in 0..maze.col_size() {
        build::print_mini_coordinate(
            maze,
            maze::Point {
                row: current_row,
                col: c,
            },
        );
    }
    print::flush();
}

fn flush_mini_carver(maze: &maze::Maze, p: maze::Point, animation: build::SpeedUnit) {
    print::set_cursor_position(
        maze::Point {
            row: p.row / 2,
            col: p.col,
        },
        maze.offset(),
    );
    if (maze.get(p.row - 1, p.col) & maze::PATH_BIT) == 0 {
        execute!(
            io::stdout(),
            SetBackgroundColor(Color::AnsiValue(CARVING)),
            Print('▀'),
            ResetColor
        )
        .expect("Printer broke");
        thread::sleep(time::Duration::from_micros(animation));
    } else {
        execute!(
            io::stdout(),
            SetForegroundColor(Color::AnsiValue(CARVING)),
            Print('▄'),
            ResetColor
        )
        .expect("Printer broke");
        thread::sleep(time::Duration::from_micros(animation));
    }
}
