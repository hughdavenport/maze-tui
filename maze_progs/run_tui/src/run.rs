use crate::tui;
use builders::build;
use crossterm::event::KeyCode;
use rand::{distributions::Bernoulli, distributions::Distribution, seq::SliceRandom, thread_rng};
use ratatui::{
    prelude::{CrosstermBackend, Rect, Terminal},
    widgets::ScrollDirection,
};
use solvers::solve;
use std::error;
use std::fmt;
use std::time::Duration;
use std::time::Instant;
use tui_textarea::{Input, Key};

#[derive(Debug)]
pub struct Quit {
    pub q: bool,
}

impl Quit {
    pub fn new() -> Self {
        Quit { q: false }
    }
}

impl fmt::Display for Quit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Quit: {}", self.q)
    }
}

#[derive(Debug, Clone)]
struct Playback {
    tape: maze::Tape,
    maze: maze::Maze,
    forward: bool,
}

impl error::Error for Quit {}

pub fn run() -> tui::Result<()> {
    let backend = CrosstermBackend::new(std::io::stdout());
    let terminal = Terminal::new(backend)?;
    let events = tui::EventHandler::new(250);
    let mut tui = tui::Tui::new(terminal, events);
    tui.enter()?;
    let mut playback = new_tape(&mut tui);
    let frame_time = Duration::from_millis(10);
    let mut last_render = Instant::now();
    'render: loop {
        if let Some(ev) = tui.events.try_next() {
            match ev {
                tui::Pack::Resize(_, _) => {
                    playback = new_tape(&mut tui);
                }
                tui::Pack::Press(ev) => {
                    match ev.into() {
                        Input { key: Key::Esc, .. } => break 'render,
                        Input { key: Key::Down, .. } => tui.scroll(ScrollDirection::Forward),
                        Input { key: Key::Up, .. } => tui.scroll(ScrollDirection::Backward),
                        Input {
                            key: Key::Enter, ..
                        } => {
                            if render_command(tui.cmd.lines()[0].to_string(), &mut tui).is_ok() {
                                tui.terminal.clear()?;
                            }
                        }
                        input => {
                            // TextArea::input returns if the input modified its text
                            let _ = tui.cmd_input(input);
                        }
                    }
                }
                tui::Pack::Tick => {}
            }
        }
        tui.home_animated(
            playback.forward,
            playback.tape.cur_step(),
            &mut playback.maze,
        )?;
        let now = Instant::now();
        if now - last_render >= frame_time {
            if playback.forward {
                playback.forward = playback.tape.move_tape_next();
            } else {
                playback.forward = !playback.tape.move_tape_prev();
            }
            last_render = now;
        }
    }
    tui.exit()?;
    Ok(())
}

fn new_tape(tui: &mut tui::Tui) -> Playback {
    let mut run_bg = set_random_args(&tui.padded_frame());
    run_bg.args.style = maze::MazeStyle::Contrast;
    let bg_maze = monitor::Solver::new(maze::Maze::new(run_bg.args));
    match bg_maze.lock() {
        Ok(mut lk) => {
            build::fill_maze_with_walls(&mut lk.maze);
        }
        Err(_) => print::maze_panic!("uncontested lock failure"),
    }
    builders::recursive_backtracker::generate_history(bg_maze.clone());
    let replay_copy = match bg_maze.lock() {
        Ok(lk) => lk.maze.clone(),
        Err(_) => print::maze_panic!("uncontested lock failure"),
    };
    solvers::bfs::hunt_history(bg_maze.clone());
    let history = match bg_maze.lock() {
        Ok(lk) => lk.maze.solve_history.to_owned(),
        Err(_) => print::maze_panic!("uncontested lock failure"),
    };
    Playback {
        tape: history,
        maze: replay_copy,
        forward: true,
    }
}

pub fn render_command(cmd: String, tui: &mut tui::Tui) -> tui::Result<()> {
    if cmd.is_empty() {
        //rand_with_channels(tui)?;
        render_maze(set_command_args(tui, cmd.as_str()).unwrap(), tui)?;
        return Ok(());
    }
    match set_command_args(tui, cmd.as_str()) {
        Ok(run) => {
            render_maze(run, tui)?;
        }
        Err(_) => return Err(Box::new(Quit::new())),
    };
    Ok(())
}

fn render_maze(mut this_run: tables::MazeRunner, tui: &mut tui::Tui) -> tui::Result<()> {
    tui.terminal.clear()?;
    //let t_start = Instant::now();
    let render_space = tui.inner_maze_rect();
    this_run.args.style = maze::MazeStyle::Sharp;
    let maze = monitor::Solver::new(maze::Maze::new(this_run.args));
    let mut replay_copy = match maze.lock() {
        Ok(mut lk) => {
            let mut maze_copy = lk.maze.clone();
            build::fill_maze_with_walls(&mut lk.maze);
            build::fill_maze_with_walls(&mut maze_copy);
            maze_copy
        }
        Err(_) => print::maze_panic!("Could not obtain lock."),
    };
    builders::recursive_backtracker::generate_history(maze.clone());
    solvers::bfs::hunt_history(maze.clone());
    // let mut quit_early = false;
    let mut playback = match maze.lock() {
        Ok(l) => l,
        Err(_) => print::maze_panic!("rendering cannot progress without lock"),
    };
    let frame_time = Duration::from_micros(2000);
    let mut last_render = Instant::now();
    let mut play_forward = true;
    'rendering: loop {
        'building: loop {
            tui.render_builder_frame(
                play_forward,
                playback.maze.build_history.cur_step(),
                &mut replay_copy,
                &render_space,
            )?;
            if tui.events.try_next().is_some() {
                break 'rendering;
            }
            let now = Instant::now();
            if now - last_render >= frame_time {
                if play_forward {
                    if !playback.maze.build_history.move_tape_next() {
                        break 'building;
                    }
                } else {
                    play_forward = !playback.maze.build_history.move_tape_prev();
                }
                last_render = now;
            }
        }
        'solving: loop {
            tui.render_solver_frame(
                play_forward,
                playback.maze.solve_history.cur_step(),
                &mut replay_copy,
                &render_space,
            )?;
            if tui.events.try_next().is_some() {
                break 'rendering;
            }
            let now = Instant::now();
            if now - last_render >= frame_time {
                if play_forward {
                    play_forward = playback.maze.solve_history.move_tape_next();
                } else if !playback.maze.solve_history.move_tape_prev() {
                    break 'solving;
                }
                last_render = now;
            }
        }
    }
    Ok(())
}

fn handle_waiting_user(
    builder: &tables::BuildFunction,
    maze: monitor::SolverMonitor,
    tui: &mut tui::Tui,
) -> tui::Result<()> {
    tui.info_prompt()?;
    let mut scroll = tui::Scroller::default();
    let mut info_popup = false;
    let description = tables::load_desc(builder);
    'looking_at_maze: loop {
        if info_popup {
            tui.info_popup(&mut scroll, description)?;
        }
        match tui.events.next()? {
            tui::Pack::Press(ke) => match ke.code {
                KeyCode::Char('i') => {
                    if info_popup {
                        if let Ok(lk) = maze.lock() {
                            tui.terminal.clear()?;
                            solve::print_paths(&lk.maze);
                            build::print_overlap_key(&lk.maze);
                            tui.info_prompt()?;
                        }
                    }
                    info_popup = !info_popup;
                }
                KeyCode::Down => {
                    if info_popup {
                        scroll.scroll(ratatui::widgets::ScrollDirection::Forward);
                    }
                }
                KeyCode::Up => {
                    if info_popup {
                        scroll.scroll(ratatui::widgets::ScrollDirection::Backward);
                    }
                }
                KeyCode::Esc => break 'looking_at_maze,
                _ => {}
            },
            tui::Pack::Resize(_, _) => break 'looking_at_maze,
            _ => {}
        }
    }
    Ok(())
}

pub fn set_command_args(tui: &mut tui::Tui, cmd: &str) -> Result<tables::MazeRunner, Quit> {
    let mut run = tables::MazeRunner::new();
    let dimensions = tui.inner_dimensions();
    run.args.odd_rows = (dimensions.rows as f64 / 1.2) as i32;
    run.args.odd_cols = dimensions.cols;
    run.args.offset = dimensions.offset;
    let mut prev_flag: &str = "";
    let mut process_current = false;
    for a in cmd.split_whitespace() {
        if process_current {
            match set_arg(
                &mut run,
                &tables::FlagArg {
                    flag: prev_flag,
                    arg: a,
                },
            ) {
                Ok(_) => {}
                Err(msg) => {
                    tui.error_popup(format!(
                        "{}\n{}\npress any key to continue",
                        msg,
                        get_arg_section(prev_flag)
                    ))
                    .expect("Tui error");
                    return Err(Quit::new());
                }
            }
            process_current = false;
            continue;
        }
        match tables::search_table(a, &tables::FLAGS) {
            Some(flag) => {
                process_current = true;
                prev_flag = flag;
            }
            None => {
                tui.error_popup(format!(
                    "unknown flag[{}].\n{}\npress any key to continue",
                    a, VALID_FLAGS
                ))
                .expect("Tui error");
                return Err(Quit::new());
            }
        }
    }
    if process_current {
        tui.error_popup(format!(
            "flag[{}] with missing arg[?]\n{}\npress any key to continue",
            prev_flag,
            get_arg_section(prev_flag)
        ))
        .expect("Tui error");
        return Err(Quit::new());
    }
    if run.args.style == maze::MazeStyle::Mini {
        run.args.odd_rows *= 2;
    }
    Ok(run)
}

fn set_arg(run: &mut tables::MazeRunner, args: &tables::FlagArg) -> Result<(), String> {
    match args.flag {
        "-b" => tables::search_table(args.arg, &tables::BUILDERS)
            .map(|func_pair| run.build = func_pair)
            .ok_or(err_string(args)),
        "-m" => tables::search_table(args.arg, &tables::MODIFICATIONS)
            .map(|mod_tuple| run.modify = Some(mod_tuple))
            .ok_or(err_string(args)),
        "-s" => tables::search_table(args.arg, &tables::SOLVERS)
            .map(|solve_tuple| run.solve = solve_tuple)
            .ok_or(err_string(args)),
        "-w" => tables::search_table(args.arg, &tables::WALL_STYLES)
            .map(|wall_style| run.args.style = wall_style)
            .ok_or(err_string(args)),
        "-ba" => match tables::search_table(args.arg, &tables::SPEEDS) {
            Some(speed) => {
                run.build_speed = speed;
                run.build_view = tables::ViewingMode::AnimatedPlayback;
                Ok(())
            }
            None => Err(err_string(args)),
        },
        "-sa" => match tables::search_table(args.arg, &tables::SPEEDS) {
            Some(speed) => {
                run.solve_speed = speed;
                run.solve_view = tables::ViewingMode::AnimatedPlayback;
                Ok(())
            }
            None => Err(err_string(args)),
        },
        _ => Err(err_string(args)),
    }
}

fn set_random_args(rect: &Rect) -> tables::MazeRunner {
    let mut rng = thread_rng();
    let mut this_run = tables::MazeRunner::new();
    this_run.build_view = tables::ViewingMode::AnimatedPlayback;
    this_run.solve_view = tables::ViewingMode::AnimatedPlayback;
    this_run.args.odd_rows = (rect.height - 1) as i32;
    this_run.args.odd_cols = (rect.width - 1) as i32;
    this_run.args.offset = maze::Offset {
        add_rows: rect.y as i32,
        add_cols: rect.x as i32,
    };
    let modification_probability = Bernoulli::new(0.2);
    this_run.args.style = match tables::WALL_STYLES.choose(&mut rng) {
        Some(&style) => style.1,
        None => print::maze_panic!("Styles not set for loop, broken"),
    };
    this_run.build_speed = match tables::SPEEDS.choose(&mut rng) {
        Some(&speed) => speed.1,
        None => print::maze_panic!("Build speed array empty."),
    };
    this_run.solve_speed = match tables::SPEEDS.choose(&mut rng) {
        Some(&speed) => speed.1,
        None => print::maze_panic!("Solve speed array empty."),
    };
    this_run.build = match tables::BUILDERS.choose(&mut rng) {
        Some(&algo) => algo.1,
        None => print::maze_panic!("Build algorithm array empty."),
    };
    this_run.solve = match tables::SOLVERS.choose(&mut rng) {
        Some(&algo) => algo.1,
        None => print::maze_panic!("Solve algorithm array empty."),
    };
    this_run.modify = None;
    if modification_probability
        .expect("Bernoulli innefective")
        .sample(&mut rng)
    {
        this_run.modify = match tables::MODIFICATIONS.choose(&mut rng) {
            Some(&m) => Some(m.1),
            None => print::maze_panic!("Modification table empty."),
        }
    }
    if this_run.args.style == maze::MazeStyle::Mini {
        this_run.args.odd_rows *= 2;
    }
    this_run
}

pub fn err_string(args: &tables::FlagArg) -> String {
    format!("invalid flag[{}] arg[{}] combo", args.flag, args.arg)
}

fn get_arg_section(flag: &str) -> &'static str {
    VALID_ARGS
        .iter()
        .find(|(f, _)| *f == flag)
        .expect("check VALID_ARGS table.")
        .1
}

pub static VALID_FLAGS: &str = "VALID FLAGS:[-b][-ba][-s][-sa][-w][-m]";
pub static VALID_ARGS: [(&str, &str); 6] = [
    ("-b", "see BUILDER FLAG section"),
    ("-m", "see MODIFICATION FLAG section"),
    ("-w", "see WALL FLAG section"),
    ("-s", "see SOLVER FLAG section"),
    ("-sa", "see SOLVER ANIMATION section"),
    ("-ba", "see BUILDER ANIMATION section"),
];
