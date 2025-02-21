mod error;
mod scenario;

#[macro_use]
mod utils_macros;

use dcapal_optimizer_wasm::{
    optimize::advanced::{self, Solution},
    JsAdvancedOptions, JsProblemOptions,
};

use glob::glob;
use log::info;
use std::{fs::File, io::BufReader, path::Path};

use error::{Error, Result};
use scenario::{Expect, ExpectedSolution, Scenario};

const SCENARIOS_PATH: &str = "./tests/scenarios";

#[test_log::test]
fn test_runner() -> anyhow::Result<()> {
    info!("==> 🚀  DcaPal Test Runner - Engine ignited");

    let pattern = if let Ok(single_test_path) = std::env::var("SINGLE_TEST") {
        single_test_path
    } else {
        let path = Path::new(SCENARIOS_PATH).canonicalize()?;
        info!("==> ⚙️  Loading test scenarios from \"{}\"", path.display());
        format!("{}/**/*.json", SCENARIOS_PATH)
    };

    let scenarios = glob(&pattern)?.filter_map(|e| e.ok()).collect::<Vec<_>>();
    for path in &scenarios {
        info!("==> 💿  Running scenario {:?}", path.file_name().unwrap());

        let scenario = read_scenario_from_file(path)?;
        let (options, expect) = scenario.split();

        // Build problem from options and solve it
        let res = match options {
            JsProblemOptions::Advanced(o) => build_solve_advanced(o),
            JsProblemOptions::Basic(_) => todo!(),
            JsProblemOptions::Analyze(_) => todo!("Analyze not implemented"),
        };

        info!("==> 🔬  Checking expectations");

        if matches!(expect, Expect::BuildError) {
            assert!(res.is_err(), "expect={expect:?} res={res:?}");
        } else {
            assert!(res.is_ok(), "expect={expect:?} res={res:?}");
        }

        match (expect, res.unwrap()) {
            (Expect::Solved(expected), TestSolution::Advanced(sol)) => {
                check_expect_advanced(&expected, &sol)
            }
            (Expect::Solved(_), TestSolution::Basic) => todo!(),
            (Expect::BuildError, _) => unreachable!(),
        }
    }

    Ok(())
}

fn read_scenario_from_file(path: &Path) -> anyhow::Result<Scenario> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let scenario: Scenario = serde_json::from_reader(reader)?;
    Ok(scenario)
}

fn build_solve_advanced(options: JsAdvancedOptions) -> Result<TestSolution> {
    let options = advanced::ProblemOptions::try_from(options).map_err(Error::BadProblemInput)?;

    let problem = advanced::Problem::new(options);
    let solution = problem.solve();

    Ok(TestSolution::Advanced(solution))
}

fn check_expect_advanced(expected: &ExpectedSolution, sol: &Solution) {
    assert!(sol.is_solved, "Expected solved problem: sol={sol:?}");

    let Some(ref expected) = expected.solution else {
        return;
    };

    for (id, exp) in expected {
        let asset = sol.assets.get(id);
        assert!(
            asset.is_some(),
            "Asset not found in solution: id={id} sol={sol:?}"
        );
        let asset = asset.unwrap();

        asset_equal_field!(id, exp, asset, shares);
        asset_equal_field!(id, exp, asset, amount);
        asset_equal_field!(id, exp, asset, weight);
    }
}

#[derive(Debug, Clone)]
pub enum TestSolution {
    Advanced(advanced::Solution),
    Basic,
}
