use rand::Rng;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::process;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Point {
    x: usize,
    y: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct State {
    cost: u32,
    position: Point,
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .cmp(&self.cost)
            .then_with(|| self.position.x.cmp(&other.position.x))
            .then_with(|| self.position.y.cmp(&other.position.y))
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct Grid {
    width: usize,
    height: usize,
    cells: Vec<u8>,
}

impl Grid {
    fn new(width: usize, height: usize) -> Self {
        Grid {
            width,
            height,
            cells: vec![0; width * height],
        }
    }

    fn get(&self, p: Point) -> u8 {
        self.cells[p.y * self.width + p.x]
    }

    fn set(&mut self, p: Point, val: u8) {
        self.cells[p.y * self.width + p.x] = val;
    }

    fn neighbors(&self, p: Point) -> Vec<Point> {
        let mut neighbors = Vec::new();
        if p.x > 0 {
            neighbors.push(Point { x: p.x - 1, y: p.y });
        }
        if p.x < self.width - 1 {
            neighbors.push(Point { x: p.x + 1, y: p.y });
        }
        if p.y > 0 {
            neighbors.push(Point { x: p.x, y: p.y - 1 });
        }
        if p.y < self.height - 1 {
            neighbors.push(Point { x: p.x, y: p.y + 1 });
        }
        neighbors
    }
}

fn generate_map(width: usize, height: usize) -> Grid {
    let mut grid = Grid::new(width, height);
    let mut rng = rand::thread_rng();

    for y in 0..height {
        for x in 0..width {
            grid.set(Point { x, y }, rng.gen());
        }
    }

    grid.set(Point { x: 0, y: 0 }, 0x00);
    grid.set(
        Point {
            x: width - 1,
            y: height - 1,
        },
        0xFF,
    );

    grid
}

fn save_map(grid: &Grid, filename: &str) -> io::Result<()> {
    let mut file = File::create(filename)?;
    for y in 0..grid.height {
        for x in 0..grid.width {
            let val = grid.get(Point { x, y });
            write!(file, "{:02X}", val)?;
            if x < grid.width - 1 {
                write!(file, " ")?;
            }
        }
        writeln!(file)?;
    }
    Ok(())
}

fn load_map(filename: &str) -> io::Result<Grid> {
    let mut file = File::open(filename)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let mut cells = Vec::new();
    let mut width = 0;
    let mut height = 0;

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let row_vals: Vec<u8> = line
            .split_whitespace()
            .map(|s| u8::from_str_radix(s, 16).unwrap_or(0))
            .collect();

        if width == 0 {
            width = row_vals.len();
        } else if row_vals.len() != width {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Inconsistent row width",
            ));
        }
        cells.extend(row_vals);
        height += 1;
    }

    Ok(Grid {
        width,
        height,
        cells,
    })
}

fn dijkstra(
    grid: &Grid,
    start: Point,
    end: Point,
    find_max: bool,
    animate: bool,
) -> Option<(u32, Vec<Point>)> {
    let mut dist: HashMap<Point, u32> = HashMap::new();
    let mut heap = BinaryHeap::new();
    let mut came_from: HashMap<Point, Point> = HashMap::new();
    let mut visited_for_anim: HashSet<Point> = HashSet::new();

    dist.insert(start, 0);
    heap.push(State {
        cost: 0,
        position: start,
    });

    let mut step_count = 0;

    while let Some(State { cost, position }) = heap.pop() {
        if animate && !visited_for_anim.contains(&position) {
            step_count += 1;
            visited_for_anim.insert(position);
            print!("\\x1b[2J\\x1b[1;1H");
            println!(
                "Searching for {} cost path...",
                if find_max { "maximum" } else { "minimum" }
            );
            println!(
                "Step {}: Exploring ({},{}) - cost: {}",
                step_count, position.x, position.y, cost
            );
            print_grid_anim(grid, &visited_for_anim, position);
            thread::sleep(Duration::from_millis(20));
        }

        if position == end {
            let mut path = Vec::new();
            let mut current = end;
            path.push(current);
            while let Some(&prev) = came_from.get(&current) {
                path.push(prev);
                current = prev;
            }
            path.reverse();

            let true_cost = if find_max {
                path.iter().map(|p| grid.get(*p) as u32).sum::<u32>()
            } else {
                cost
            };

            return Some((true_cost, path));
        }

        if cost > *dist.get(&position).unwrap_or(&u32::MAX) {
            continue;
        }

        for neighbor in grid.neighbors(position) {
            let val = grid.get(neighbor);
            let weight = if find_max {
                255 - val as u32
            } else {
                val as u32
            };

            let next_cost = cost + weight;

            if next_cost < *dist.get(&neighbor).unwrap_or(&u32::MAX) {
                heap.push(State {
                    cost: next_cost,
                    position: neighbor,
                });
                dist.insert(neighbor, next_cost);
                came_from.insert(neighbor, position);
            }
        }
    }

    None
}

fn print_grid_anim(grid: &Grid, visited: &HashSet<Point>, current: Point) {
    for y in 0..grid.height {
        for x in 0..grid.width {
            let p = Point { x, y };
            if p == current {
                print!("[*]");
            } else if visited.contains(&p) {
                print!("[✓]");
            } else {
                print!("[ ]");
            }
        }
        println!();
    }
}

fn print_path_details(grid: &Grid, path: &[Point], total_cost: u32, title: &str) {
    println!("\\n{}:", title);
    println!("==================");
    println!("Total cost: 0x{:X} ({} decimal)", total_cost, total_cost);
    println!("Path length: {} steps", path.len());

    print!("Path: ");
    for (i, p) in path.iter().enumerate() {
        print!("({},{})", p.x, p.y);
        if i < path.len() - 1 {
            print!("→");
        }
    }
    println!();

    println!("\\nStep-by-step costs:");
    let mut current_cost = 0;
    for (i, p) in path.iter().enumerate() {
        let val = grid.get(*p);
        if i == 0 {
            println!("  Start  0x{:02X} ({},{})", val, p.x, p.y);
            current_cost += val as u32;
        } else {
            current_cost += val as u32;
            println!("    →    0x{:02X} ({},{})  +{}", val, p.x, p.y, val);
        }
    }
    println!("  Total: 0x{:X} ({})", current_cost, current_cost);
}

fn visualize_grid(grid: &Grid, min_path: Option<&[Point]>, max_path: Option<&[Point]>) {
    let min_set: HashSet<Point> = min_path.unwrap_or(&[]).iter().cloned().collect();
    let max_set: HashSet<Point> = max_path.unwrap_or(&[]).iter().cloned().collect();

    for y in 0..grid.height {
        for x in 0..grid.width {
            let p = Point { x, y };
            let val = grid.get(p);

            let (r, g, b) = if val < 128 {
                (255, (val as u16 * 2) as u8, 0)
            } else {
                (255, 255, ((val as u16 - 128) * 2) as u8)
            };

            let bg_code = if min_set.contains(&p) && max_set.contains(&p) {
                "\\x1b[48;2;255;0;255m"
            } else if min_set.contains(&p) {
                "\\x1b[48;2;255;255;255m\\x1b[30m"
            } else if max_set.contains(&p) {
                "\\x1b[48;2;255;0;0m\\x1b[37m"
            } else {
                ""
            };

            if min_set.contains(&p) || max_set.contains(&p) {
                print!("{}{:02X}\\x1b[0m ", bg_code, val);
            } else {
                print!("\\x1b[38;2;{};{};{}m{:02X}\\x1b[0m ", r, g, b, val);
            }
        }
        println!();
    }
}

fn print_help() {
    println!("Usage: hexpath [OPTIONS] <map>");
    println!("\\nFind min/max cost paths in hexadecimal grid");
    println!("\\nArguments:");
    println!("  <map>  Map file (hex values, space separated)");
    println!("\\nOptions:");
    println!("  --generate <widthxheight>  Generate random map (e.g., 8x4, 10x10)");
    println!("  --output <file>            Save generated map to file");
    println!("  --visualize                Show colored map");
    println!("  --both                     Show both min and max paths");
    println!("  --animate                  Animate pathfinding");
    println!("  -h, --help");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut generate_size: Option<(usize, usize)> = None;
    let mut output_file: Option<String> = None;
    let mut input_file: Option<String> = None;
    let mut visualize = false;
    let mut animate = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_help();
                return;
            }
            "--generate" => {
                if i + 1 < args.len() {
                    let parts: Vec<&str> = args[i + 1].split('x').collect();
                    if parts.len() == 2 {
                        if let (Ok(w), Ok(h)) = (parts[0].parse(), parts[1].parse()) {
                            generate_size = Some((w, h));
                        }
                    }
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--output" => {
                if i + 1 < args.len() {
                    output_file = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--visualize" => {
                visualize = true;
                i += 1;
            }
            "--both" => {
                i += 1;
            }
            "--animate" => {
                animate = true;
                i += 1;
            }
            arg => {
                if !arg.starts_with("-") {
                    input_file = Some(arg.to_string());
                }
                i += 1;
            }
        }
    }

    if let Some((w, h)) = generate_size {
        println!("Generating {}x{} hexadecimal grid...", w, h);
        let grid = generate_map(w, h);

        if let Some(filename) = output_file {
            if let Err(e) = save_map(&grid, &filename) {
                eprintln!("Error saving map: {}", e);
                process::exit(1);
            }
            println!("Map saved to: {}", filename);
        }

        println!("\\nGenerated map:");
        for y in 0..grid.height {
            for x in 0..grid.width {
                print!("{:02X} ", grid.get(Point { x, y }));
            }
            println!();
        }
        return;
    }

    let filename = match input_file {
        Some(f) => f,
        None => {
            if args.len() > 1 {
                eprintln!("Error: No map file specified");
                process::exit(1);
            }
            print_help();
            return;
        }
    };

    let grid = match load_map(&filename) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Error loading map: {}", e);
            process::exit(1);
        }
    };

    if animate {
        println!("Searching for minimum cost path...");
        dijkstra(
            &grid,
            Point { x: 0, y: 0 },
            Point {
                x: grid.width - 1,
                y: grid.height - 1,
            },
            false,
            true,
        );
        println!("Done.");
        return;
    }

    if visualize {
        let start = Point { x: 0, y: 0 };
        let end = Point {
            x: grid.width - 1,
            y: grid.height - 1,
        };
        let min_res = dijkstra(&grid, start, end, false, false);
        visualize_grid(&grid, min_res.as_ref().map(|r| r.1.as_slice()), None);
        return;
    }

    println!("Analyzing hexadecimal grid...");
    println!("Grid size: {}×{}", grid.width, grid.height);
    println!("Start: (0,0) = 0x{:02X}", grid.get(Point { x: 0, y: 0 }));
    println!(
        "End: ({},{}) = 0x{:02X}",
        grid.width - 1,
        grid.height - 1,
        grid.get(Point {
            x: grid.width - 1,
            y: grid.height - 1
        })
    );

    let start = Point { x: 0, y: 0 };
    let end = Point {
        x: grid.width - 1,
        y: grid.height - 1,
    };

    let min_result = dijkstra(&grid, start, end, false, false);
    if let Some((cost, ref path)) = min_result {
        print_path_details(&grid, path, cost, "MINIMUM COST PATH");
    } else {
        println!("No minimum path found!");
    }

    let max_result = dijkstra(&grid, start, end, true, false);
    if let Some((cost, ref path)) = max_result {
        print_path_details(&grid, path, cost, "MAXIMUM COST PATH");
    } else {
        println!("No maximum path found!");
    }
}
