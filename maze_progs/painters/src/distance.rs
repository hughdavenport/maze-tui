use crate::rgb;
use builders::build::print_square;
use maze;
use solvers::solve;
use std::collections::{HashSet, VecDeque};

use std::{thread, time};

use rand::{thread_rng, Rng};

pub fn paint_distance_from_center(monitor: solve::SolverMonitor) {
    let mut lk = match monitor.lock() {
        Ok(l) => l,
        Err(_) => print::maze_panic!("Lock panic."),
    };

    let row_mid = lk.maze.row_size() / 2;
    let col_mid = lk.maze.col_size() / 2;
    let start = maze::Point {
        row: row_mid + 1 - (row_mid % 2),
        col: col_mid + 1 - (col_mid % 2),
    };
    let mut map = solve::MaxMap::new(start, 0);
    let mut bfs = VecDeque::from([(start, 0u64)]);
    lk.maze[start.row as usize][start.col as usize] |= rgb::MEASURE;
    while let Some(cur) = bfs.pop_front() {
        if cur.1 > map.max {
            map.max = cur.1;
        }
        for &p in maze::CARDINAL_DIRECTIONS.iter() {
            let next = maze::Point {
                row: cur.0.row + p.row,
                col: cur.0.col + p.col,
            };
            if (lk.maze[next.row as usize][next.col as usize] & maze::PATH_BIT) == 0
                || (lk.maze[next.row as usize][next.col as usize] & rgb::MEASURE) != 0
            {
                continue;
            }
            lk.maze[next.row as usize][next.col as usize] |= rgb::MEASURE;
            map.distances.insert(next, cur.1 + 1);
            bfs.push_back((next, cur.1 + 1));
        }
    }
    painter(&mut lk.maze, &map);
}

pub fn animate_distance_from_center(monitor: solve::SolverMonitor, speed: speed::Speed) {
    if monitor
        .lock()
        .unwrap_or_else(|_| print::maze_panic!("Thread panicked"))
        .maze
        .is_mini()
    {
        animate_mini_distance_from_center(monitor, speed);
        return;
    }
    let start = if let Ok(mut lk) = monitor.lock() {
        let row_mid = lk.maze.row_size() / 2;
        let col_mid = lk.maze.col_size() / 2;
        let start = maze::Point {
            row: row_mid + 1 - (row_mid % 2),
            col: col_mid + 1 - (col_mid % 2),
        };
        lk.map.distances.insert(start, 0);
        let mut bfs = VecDeque::from([(start, 0u64)]);
        lk.maze[start.row as usize][start.col as usize] |= rgb::MEASURE;
        while let Some(cur) = bfs.pop_front() {
            if lk.maze.exit() {
                return;
            }
            if cur.1 > lk.map.max {
                lk.map.max = cur.1;
            }
            for &p in maze::CARDINAL_DIRECTIONS.iter() {
                let next = maze::Point {
                    row: cur.0.row + p.row,
                    col: cur.0.col + p.col,
                };
                if (lk.maze[next.row as usize][next.col as usize] & maze::PATH_BIT) == 0
                    || (lk.maze[next.row as usize][next.col as usize] & rgb::MEASURE) != 0
                {
                    continue;
                }
                lk.maze[next.row as usize][next.col as usize] |= rgb::MEASURE;
                lk.map.distances.insert(next, cur.1 + 1);
                bfs.push_back((next, cur.1 + 1));
            }
        }
        start
    } else {
        print::maze_panic!("Thread panic.");
    };

    let mut rng = thread_rng();
    let rand_color_choice: usize = rng.gen_range(0..3);
    let mut handles = Vec::with_capacity(rgb::NUM_PAINTERS);
    let animation = rgb::ANIMATION_SPEEDS[speed as usize];
    for painter in 0..rgb::NUM_PAINTERS {
        let monitor_clone = monitor.clone();
        handles.push(thread::spawn(move || {
            painter_animated(
                monitor_clone,
                rgb::ThreadGuide {
                    bias: painter,
                    color_i: rand_color_choice,
                    p: start,
                },
                animation,
            );
        }));
    }
    for h in handles {
        h.join().expect("Error joining a thread.");
    }
}

fn animate_mini_distance_from_center(monitor: solve::SolverMonitor, speed: speed::Speed) {
    let start = if let Ok(mut lk) = monitor.lock() {
        let row_mid = lk.maze.row_size() / 2;
        let col_mid = lk.maze.col_size() / 2;
        let start = maze::Point {
            row: row_mid + 1 - (row_mid % 2),
            col: col_mid + 1 - (col_mid % 2),
        };
        lk.map.distances.insert(start, 0);
        let mut bfs = VecDeque::from([(start, 0u64)]);
        lk.maze[start.row as usize][start.col as usize] |= rgb::MEASURE;
        while let Some(cur) = bfs.pop_front() {
            if lk.maze.exit() {
                return;
            }
            if cur.1 > lk.map.max {
                lk.map.max = cur.1;
            }
            for &p in maze::CARDINAL_DIRECTIONS.iter() {
                let next = maze::Point {
                    row: cur.0.row + p.row,
                    col: cur.0.col + p.col,
                };
                if (lk.maze[next.row as usize][next.col as usize] & maze::PATH_BIT) == 0
                    || (lk.maze[next.row as usize][next.col as usize] & rgb::MEASURE) != 0
                {
                    continue;
                }
                lk.maze[next.row as usize][next.col as usize] |= rgb::MEASURE;
                lk.map.distances.insert(next, cur.1 + 1);
                bfs.push_back((next, cur.1 + 1));
            }
        }
        start
    } else {
        print::maze_panic!("Thread panic.");
    };

    let mut rng = thread_rng();
    let rand_color_choice: usize = rng.gen_range(0..3);
    let mut handles = Vec::with_capacity(rgb::NUM_PAINTERS);
    let animation = rgb::ANIMATION_SPEEDS[speed as usize];
    for painter in 0..rgb::NUM_PAINTERS - 1 {
        let monitor_clone = monitor.clone();
        handles.push(thread::spawn(move || {
            painter_mini_animated(
                monitor_clone,
                rgb::ThreadGuide {
                    bias: painter,
                    color_i: rand_color_choice,
                    p: start,
                },
                animation,
            );
        }));
    }
    painter_mini_animated(
        monitor.clone(),
        rgb::ThreadGuide {
            bias: rgb::NUM_PAINTERS - 1,
            color_i: rand_color_choice,
            p: start,
        },
        animation,
    );
    for h in handles {
        h.join().expect("Error joining a thread.");
    }
}

// Private Helper Functions-----------------------------------------------------------------------

fn painter(maze: &maze::Maze, map: &solve::MaxMap) {
    let mut rng = thread_rng();
    let rand_color_choice: usize = rng.gen_range(0..3);
    if maze.style_index() == (maze::MazeStyle::Mini as usize) {
        for r in 0..maze.row_size() {
            for c in 0..maze.col_size() {
                let cur = maze::Point { row: r, col: c };
                if (maze[r as usize][c as usize] & maze::PATH_BIT) == 0 {
                    solve::print_mini_point(maze, cur);
                    continue;
                }
                let dist = map.distances.get(&cur).expect("Could not find map entry?");
                let intensity = (map.max - dist) as f64 / map.max as f64;
                let dark = (255f64 * intensity) as u8;
                let bright = 128 + (127f64 * intensity) as u8;
                let mut channels: rgb::Rgb = [dark, dark, dark];
                channels[rand_color_choice] = bright;
                if (maze[(cur.row + 1) as usize][cur.col as usize] & maze::PATH_BIT) != 0 {
                    let neighbor = maze::Point {
                        row: cur.row + 1,
                        col: cur.col,
                    };
                    let neighbor_dist = map.distances.get(&neighbor).expect("Empty map");
                    let intensity = (map.max - neighbor_dist) as f64 / map.max as f64;
                    let dark = (255f64 * intensity) as u8;
                    let bright = 128 + (127f64 * intensity) as u8;
                    let mut neighbor_channels: rgb::Rgb = [dark, dark, dark];
                    neighbor_channels[rand_color_choice] = bright;
                    rgb::print_mini_rgb(
                        Some(channels),
                        Some(neighbor_channels),
                        cur,
                        maze.offset(),
                    );
                } else {
                    rgb::print_mini_rgb(Some(channels), None, cur, maze.offset());
                }
            }
        }
    } else {
        for r in 0..maze.row_size() {
            for c in 0..maze.col_size() {
                let cur = maze::Point { row: r, col: c };
                match map.distances.get(&cur) {
                    Some(dist) => {
                        let intensity = (map.max - dist) as f64 / map.max as f64;
                        let dark = (255f64 * intensity) as u8;
                        let bright = 128 + (127f64 * intensity) as u8;
                        let mut channels: rgb::Rgb = [dark, dark, dark];
                        channels[rand_color_choice] = bright;
                        rgb::print_rgb(channels, cur, maze.offset());
                    }
                    None => print_square(&maze, cur),
                }
            }
        }
    }
}

fn painter_animated(
    monitor: solve::SolverMonitor,
    guide: rgb::ThreadGuide,
    animation: rgb::SpeedUnit,
) {
    let mut seen = HashSet::from([guide.p]);
    let mut bfs = VecDeque::from([guide.p]);
    while let Some(cur) = bfs.pop_front() {
        match monitor.lock() {
            Ok(mut lk) => {
                if lk.maze.exit() || lk.count == lk.map.distances.len() {
                    return;
                }
                let dist = lk
                    .map
                    .distances
                    .get(&cur)
                    .expect("Could not find map entry?");
                if (lk.maze[cur.row as usize][cur.col as usize] & rgb::PAINT) == 0 {
                    let intensity = (lk.map.max - dist) as f64 / lk.map.max as f64;
                    let dark = (255f64 * intensity) as u8;
                    let bright = 128 + (127f64 * intensity) as u8;
                    let mut channels: rgb::Rgb = [dark, dark, dark];
                    channels[guide.color_i] = bright;
                    rgb::animate_rgb(channels, cur, lk.maze.offset());
                    lk.maze[cur.row as usize][cur.col as usize] |= rgb::PAINT;
                    lk.count += 1;
                }
            }
            Err(p) => print::maze_panic!("Thread panicked with lock: {}", p),
        };
        thread::sleep(time::Duration::from_micros(animation));
        let mut i = guide.bias;
        while {
            let p = &maze::CARDINAL_DIRECTIONS[i];
            let next = maze::Point {
                row: cur.row + p.row,
                col: cur.col + p.col,
            };
            if match monitor.lock() {
                Err(p) => print::maze_panic!("Panic with lock: {}", p),
                Ok(lk) => (lk.maze[next.row as usize][next.col as usize] & maze::PATH_BIT) != 0,
            } && !seen.contains(&next)
            {
                seen.insert(next);
                bfs.push_back(next);
            }
            i = (i + 1) % rgb::NUM_PAINTERS;
            i != guide.bias
        } {}
    }
}

fn painter_mini_animated(
    monitor: solve::SolverMonitor,
    guide: rgb::ThreadGuide,
    animation: rgb::SpeedUnit,
) {
    let mut seen = HashSet::from([guide.p]);
    let mut bfs = VecDeque::from([guide.p]);
    while let Some(cur) = bfs.pop_front() {
        match monitor.lock() {
            Ok(mut lk) => {
                if lk.maze.exit() || lk.count == lk.map.distances.len() {
                    return;
                }
                if (lk.maze[cur.row as usize][cur.col as usize] & rgb::PAINT) == 0 {
                    let dist = lk
                        .map
                        .distances
                        .get(&cur)
                        .expect("Could not find map entry?");
                    let intensity = (lk.map.max - dist) as f64 / lk.map.max as f64;
                    let dark = (255f64 * intensity) as u8;
                    let bright = 128 + (127f64 * intensity) as u8;
                    let mut channels: rgb::Rgb = [dark, dark, dark];
                    channels[guide.color_i] = bright;
                    lk.maze[cur.row as usize][cur.col as usize] |= rgb::PAINT;
                    lk.count += 1;
                    if cur.row % 2 == 0 {
                        if (lk.maze[(cur.row + 1) as usize][cur.col as usize] & maze::PATH_BIT) != 0
                        {
                            let neighbor = maze::Point {
                                row: cur.row + 1,
                                col: cur.col,
                            };
                            lk.maze[neighbor.row as usize][neighbor.col as usize] |= rgb::PAINT;
                            let neighbor_dist = lk.map.distances.get(&neighbor).expect("Empty map");
                            let intensity = (lk.map.max - neighbor_dist) as f64 / lk.map.max as f64;
                            let dark = (255f64 * intensity) as u8;
                            let bright = 128 + (127f64 * intensity) as u8;
                            let mut neighbor_channels: rgb::Rgb = [dark, dark, dark];
                            neighbor_channels[guide.color_i] = bright;
                            rgb::animate_mini_rgb(
                                Some(channels),
                                Some(neighbor_channels),
                                cur,
                                lk.maze.offset(),
                            );
                        } else {
                            rgb::animate_mini_rgb(Some(channels), None, cur, lk.maze.offset());
                        }
                    } else {
                        if (lk.maze[(cur.row - 1) as usize][cur.col as usize] & maze::PATH_BIT) != 0
                        {
                            let neighbor = maze::Point {
                                row: cur.row - 1,
                                col: cur.col,
                            };
                            lk.maze[neighbor.row as usize][neighbor.col as usize] |= rgb::PAINT;
                            let neighbor_dist = lk.map.distances.get(&neighbor).expect("Empty map");
                            let intensity = (lk.map.max - neighbor_dist) as f64 / lk.map.max as f64;
                            let dark = (255f64 * intensity) as u8;
                            let bright = 128 + (127f64 * intensity) as u8;
                            let mut neighbor_channels: rgb::Rgb = [dark, dark, dark];
                            neighbor_channels[guide.color_i] = bright;
                            rgb::animate_mini_rgb(
                                Some(neighbor_channels),
                                Some(channels),
                                cur,
                                lk.maze.offset(),
                            );
                        } else {
                            rgb::animate_mini_rgb(None, Some(channels), cur, lk.maze.offset());
                        }
                    }
                }
            }
            Err(p) => print::maze_panic!("Thread panicked with lock: {}", p),
        }

        thread::sleep(time::Duration::from_micros(animation));
        let mut i = guide.bias;
        while {
            let p = &maze::CARDINAL_DIRECTIONS[i];
            let next = maze::Point {
                row: cur.row + p.row,
                col: cur.col + p.col,
            };
            if match monitor.lock() {
                Err(p) => print::maze_panic!("Panic with lock: {}", p),
                Ok(lk) => (lk.maze[next.row as usize][next.col as usize] & maze::PATH_BIT) != 0,
            } && !seen.contains(&next)
            {
                seen.insert(next);
                bfs.push_back(next);
            }
            i = (i + 1) % rgb::NUM_PAINTERS;
            i != guide.bias
        } {}
    }
}
