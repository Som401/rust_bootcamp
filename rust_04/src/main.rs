use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashSet};
use std::env;
use std::fs;
use std::io::Write;
use std::thread;
use std::time::Duration;

struct SimplePrng {
    state: u64,
}

impl SimplePrng {
    fn new() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        SimplePrng { state: seed }
    }

    fn gen_range(&mut self, min: u8, max: u8) -> u8 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        let range = (max - min) as u64 + 1;
        min + ((self.state % range) as u8)
    }
}

#[derive(Clone)]
struct Grid {
    cells: Vec<Vec<u8>>,
    width: usize,
    height: usize,
}

struct PathResult {
    path: Vec<(usize, usize)>,
    total_cost: u32,
}

impl Grid {
    fn new(cells: Vec<Vec<u8>>) -> Self {
        let height = cells.len();
        let width = if height > 0 { cells[0].len() } else { 0 };
        Grid {
            cells,
            width,
            height,
        }
    }

    fn get(&self, x: usize, y: usize) -> u8 {
        self.cells[y][x]
    }

    fn neighbors(&self, x: usize, y: usize) -> Vec<(usize, usize)> {
        let mut result = Vec::new();
        if x > 0 {
            result.push((x - 1, y));
        }
        if x < self.width - 1 {
            result.push((x + 1, y));
        }
        if y > 0 {
            result.push((x, y - 1));
        }
        if y < self.height - 1 {
            result.push((x, y + 1));
        }
        result
    }
}

fn generate_map(width: usize, height: usize) -> Grid {
    let mut rng = SimplePrng::new();
    let mut cells = vec![vec![0u8; width]; height];

    cells[0][0] = 0x00;
    cells[height - 1][width - 1] = 0xFF;

    for (y, row) in cells.iter_mut().enumerate() {
        for (x, cell) in row.iter_mut().enumerate() {
            if (x, y) == (0, 0) || (x, y) == (width - 1, height - 1) {
                continue;
            }
            *cell = rng.gen_range(0x01, 0xFE);
        }
    }

    Grid::new(cells)
}

fn parse_map(path: &str) -> Result<Grid, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let mut cells = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let row: Result<Vec<u8>, _> = line
            .split_whitespace()
            .map(|s| u8::from_str_radix(s, 16))
            .collect();

        cells.push(row.map_err(|e| format!("Invalid hex value: {}", e))?);
    }

    if cells.is_empty() {
        return Err("Empty map".to_string());
    }

    let width = cells[0].len();
    for row in &cells {
        if row.len() != width {
            return Err("Inconsistent row lengths".to_string());
        }
    }

    Ok(Grid::new(cells))
}

fn save_map(grid: &Grid, path: &str) -> Result<(), String> {
    let mut file = fs::File::create(path).map_err(|e| format!("Failed to create file: {}", e))?;

    for row in &grid.cells {
        let line: Vec<String> = row.iter().map(|&v| format!("{:02X}", v)).collect();
        writeln!(file, "{}", line.join(" ")).map_err(|e| format!("Failed to write: {}", e))?;
    }

    Ok(())
}

fn dijkstra_min(grid: &Grid) -> Option<PathResult> {
    let mut heap = BinaryHeap::new();
    let mut dist = vec![vec![u32::MAX; grid.width]; grid.height];
    let mut parent = vec![vec![None; grid.width]; grid.height];

    heap.push(Reverse((0u32, 0usize, 0usize)));
    dist[0][0] = 0;

    while let Some(Reverse((cost, x, y))) = heap.pop() {
        if (x, y) == (grid.width - 1, grid.height - 1) {
            return Some(reconstruct_path(grid, &parent, &dist, false));
        }

        if cost > dist[y][x] {
            continue;
        }

        for (nx, ny) in grid.neighbors(x, y) {
            let new_cost = cost + grid.get(nx, ny) as u32;
            if new_cost < dist[ny][nx] {
                dist[ny][nx] = new_cost;
                parent[ny][nx] = Some((x, y));
                heap.push(Reverse((new_cost, nx, ny)));
            }
        }
    }

    None
}

fn dijkstra_max(grid: &Grid) -> Option<PathResult> {
    let mut heap = BinaryHeap::new();
    let mut dist = vec![vec![0u32; grid.width]; grid.height];
    let mut parent = vec![vec![None; grid.width]; grid.height];
    let mut visited = vec![vec![false; grid.width]; grid.height];

    heap.push((0u32, 0usize, 0usize));
    dist[0][0] = 0;

    while let Some((cost, x, y)) = heap.pop() {
        if visited[y][x] {
            continue;
        }
        visited[y][x] = true;

        if (x, y) == (grid.width - 1, grid.height - 1) {
            return Some(reconstruct_path(grid, &parent, &dist, true));
        }

        for (nx, ny) in grid.neighbors(x, y) {
            if !visited[ny][nx] {
                let new_cost = cost + grid.get(nx, ny) as u32;
                if new_cost > dist[ny][nx] {
                    dist[ny][nx] = new_cost;
                    parent[ny][nx] = Some((x, y));
                    heap.push((new_cost, nx, ny));
                }
            }
        }
    }

    None
}

fn reconstruct_path(
    grid: &Grid,
    parent: &[Vec<Option<(usize, usize)>>],
    dist: &[Vec<u32>],
    _is_max: bool,
) -> PathResult {
    let mut path = Vec::new();
    let mut current = (grid.width - 1, grid.height - 1);

    while let Some((x, y)) = Some(current) {
        path.push((x, y));

        if (x, y) == (0, 0) {
            break;
        }

        if let Some(p) = parent[y][x] {
            current = p;
        } else {
            break;
        }
    }

    path.reverse();

    let total_cost = dist[grid.height - 1][grid.width - 1];

    PathResult { path, total_cost }
}

fn get_color(value: u8) -> &'static str {
    match value {
        0x00..=0x1F => "\x1b[38;5;196m",
        0x20..=0x3F => "\x1b[38;5;208m",
        0x40..=0x5F => "\x1b[38;5;226m",
        0x60..=0x7F => "\x1b[38;5;46m",
        0x80..=0x9F => "\x1b[38;5;51m",
        0xA0..=0xBF => "\x1b[38;5;21m",
        0xC0..=0xDF => "\x1b[38;5;129m",
        0xE0..=0xFF => "\x1b[38;5;201m",
    }
}

fn visualize_grid(grid: &Grid, min_path: Option<&PathResult>, max_path: Option<&PathResult>) {
    let min_set: HashSet<_> = min_path
        .map(|p| p.path.iter().cloned().collect())
        .unwrap_or_default();
    let max_set: HashSet<_> = max_path
        .map(|p| p.path.iter().cloned().collect())
        .unwrap_or_default();

    println!("\nHEXADECIMAL GRID (rainbow gradient):");
    println!("═══════════════════════════════════════════════════════════════════════════════");
    for y in 0..grid.height {
        for x in 0..grid.width {
            let value = grid.get(x, y);
            let color = get_color(value);
            print!("{}{:02X}\x1b[0m ", color, value);
        }
        println!();
    }

    if min_path.is_some() {
        println!("\nMINIMUM COST PATH (shown in WHITE):");
        println!("═══════════════════════════════════");
        for y in 0..grid.height {
            for x in 0..grid.width {
                let value = grid.get(x, y);
                if min_set.contains(&(x, y)) {
                    print!("\x1b[47m\x1b[30m{:02X}\x1b[0m ", value);
                } else {
                    let color = get_color(value);
                    print!("{}{:02X}\x1b[0m ", color, value);
                }
            }
            println!();
        }
        if let Some(min) = min_path {
            println!("\nCost: {} (minimum)", min.total_cost);
        }
    }

    if max_path.is_some() {
        println!("\nMAXIMUM COST PATH (shown in RED):");
        println!("═════════════════════════════════");
        for y in 0..grid.height {
            for x in 0..grid.width {
                let value = grid.get(x, y);
                if max_set.contains(&(x, y)) {
                    print!("\x1b[41m\x1b[37m{:02X}\x1b[0m ", value);
                } else {
                    let color = get_color(value);
                    print!("{}{:02X}\x1b[0m ", color, value);
                }
            }
            println!();
        }
        if let Some(max) = max_path {
            println!("\nCost: {} (maximum)", max.total_cost);
        }
    }
}

fn print_path_analysis(grid: &Grid, result: &PathResult, label: &str) {
    println!("\n{} COST PATH:", label);
    println!("==================");
    println!(
        "Total cost: 0x{:X} ({} decimal)",
        result.total_cost, result.total_cost
    );
    println!("Path length: {} steps", result.path.len());

    let path_str: Vec<String> = result
        .path
        .iter()
        .map(|(x, y)| format!("({},{})", x, y))
        .collect();
    println!("Path: {}", path_str.join("→"));

    println!("\nStep-by-step costs:");
    for (i, &(x, y)) in result.path.iter().enumerate() {
        let value = grid.get(x, y);
        if i == 0 {
            println!("  Start  0x{:02X} ({},{})", value, x, y);
        } else {
            println!("    →    0x{:02X} ({},{})  +{}", value, x, y, value);
        }
    }
    println!("  Total: 0x{:X} ({})", result.total_cost, result.total_cost);
}

fn animate_pathfinding(grid: &Grid) {
    println!("Searching for minimum cost path...\n");

    let mut heap = BinaryHeap::new();
    let mut dist = vec![vec![u32::MAX; grid.width]; grid.height];
    let mut visited = vec![vec![false; grid.width]; grid.height];

    heap.push(Reverse((0u32, 0usize, 0usize)));
    dist[0][0] = 0;

    let mut step = 0;

    while let Some(Reverse((cost, x, y))) = heap.pop() {
        if visited[y][x] {
            continue;
        }
        visited[y][x] = true;
        step += 1;

        println!("Step {}: Exploring ({},{}) - cost: {}", step, x, y, cost);

        for (row_y, row) in visited.iter().enumerate() {
            for (col_x, &is_visited) in row.iter().enumerate() {
                if is_visited {
                    print!("[✓]");
                } else if (col_x, row_y) == (x, y) {
                    print!("[*]");
                } else {
                    print!("[ ]");
                }
            }
            println!();
        }
        println!();
        thread::sleep(Duration::from_millis(200));

        if (x, y) == (grid.width - 1, grid.height - 1) {
            println!("✓ Reached destination!");
            break;
        }

        for (nx, ny) in grid.neighbors(x, y) {
            if !visited[ny][nx] {
                let new_cost = cost + grid.get(nx, ny) as u32;
                if new_cost < dist[ny][nx] {
                    dist[ny][nx] = new_cost;
                    heap.push(Reverse((new_cost, nx, ny)));
                }
            }
        }
    }
}

fn print_help() {
    println!("hexpath");
    println!();
    println!("Find min/max cost paths in hexadecimal grid");
    println!();
    println!("Map format:");
    println!("  - Each cell: 00-FF (hexadecimal)");
    println!("  - Start: top-left (must be 00)");
    println!("  - End: bottom-right (must be FF)");
    println!("  - Moves: up, down, left, right");
    println!();
    println!("Usage: hexpath [OPTIONS] [map]");
    println!();
    println!("Arguments:");
    println!("  [map]  Map file (hex values, space separated)");
    println!();
    println!("Options:");
    println!("      --generate <widthxheight>  Generate random map (e.g., 8x4, 10x10)");
    println!("      --output <file>            Save generated map to file");
    println!("      --visualize                Show colored map");
    println!("      --both                     Show both min and max paths");
    println!("      --animate                  Animate pathfinding");
    println!("  -h, --help                     Print help");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut generate_spec: Option<String> = None;
    let mut output_path: Option<String> = None;
    let mut map_path: Option<String> = None;
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
                    generate_spec = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --generate requires a value");
                    std::process::exit(1);
                }
            }
            "--output" => {
                if i + 1 < args.len() {
                    output_path = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --output requires a value");
                    std::process::exit(1);
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
                if arg.starts_with('-') {
                    eprintln!("error: Invalid option: {}", arg);
                    std::process::exit(2);
                } else {
                    map_path = Some(arg.to_string());
                }
                i += 1;
            }
        }
    }

    if let Some(gen_spec) = &generate_spec {
        let parts: Vec<&str> = gen_spec.split('x').collect();
        if parts.len() != 2 {
            eprintln!("Error: Invalid format. Use WIDTHxHEIGHT (e.g., 12x8)");
            std::process::exit(1);
        }

        let width: usize = parts[0].parse().unwrap_or_else(|_| {
            eprintln!("Error: Invalid width");
            std::process::exit(1);
        });

        let height: usize = parts[1].parse().unwrap_or_else(|_| {
            eprintln!("Error: Invalid height");
            std::process::exit(1);
        });

        println!("Generating {}x{} hexadecimal grid...", width, height);
        let grid = generate_map(width, height);

        if let Some(output) = &output_path {
            if let Err(e) = save_map(&grid, output) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
            println!("Map saved to: {}", output);
        }

        println!("\nGenerated map:");
        for row in &grid.cells {
            let line: Vec<String> = row.iter().map(|&v| format!("{:02X}", v)).collect();
            println!("{}", line.join(" "));
        }

        return;
    }

    let map = map_path.as_ref().unwrap_or_else(|| {
        eprintln!("Error: Map file required (or use --generate)");
        std::process::exit(1);
    });

    let grid = match parse_map(map) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    if animate {
        animate_pathfinding(&grid);
        return;
    }

    let min_result = dijkstra_min(&grid);
    let max_result = dijkstra_max(&grid);

    if visualize {
        visualize_grid(&grid, min_result.as_ref(), max_result.as_ref());
    } else {
        println!("Analyzing hexadecimal grid...");
        println!("Grid size: {}×{}", grid.width, grid.height);
        println!("Start: (0,0) = 0x{:02X}", grid.get(0, 0));
        println!(
            "End: ({},{}) = 0x{:02X}",
            grid.width - 1,
            grid.height - 1,
            grid.get(grid.width - 1, grid.height - 1)
        );

        if let Some(ref min) = min_result {
            print_path_analysis(&grid, min, "MINIMUM");
        }

        if let Some(ref max) = max_result {
            print_path_analysis(&grid, max, "MAXIMUM");
        }
    }
}
