#[cfg(test)]
mod tests {
    use eyre::Result;
    use log::error;
    use plotters::prelude::*;
    use rand::SeedableRng;
    use rand_distr::{Distribution, Normal};
    use rand_xorshift::XorShiftRng;
    #[test]
    fn test_plot() -> Result<()> {
        let root = BitMapBackend::new("1.png", (640, 480)).into_drawing_area();
        root.fill(&WHITE)?;
        let mut chart = ChartBuilder::on(&root)
            .caption("y=x^2", ("sans-serif", 50).into_font())
            .margin(5)
            .x_label_area_size(30)
            .y_label_area_size(30)
            .build_cartesian_2d(-1f32..1f32, -0.1f32..1f32)?;

        chart.configure_mesh().draw()?;

        chart
            .draw_series(LineSeries::new(
                (-50..=50).map(|x| x as f32 / 50.0).map(|x| (x, x * x)),
                &RED,
            ))?
            .label("y = x^2")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

        chart
            .configure_series_labels()
            .background_style(&WHITE.mix(0.8))
            .border_style(&BLACK)
            .draw()?;

        root.present()?;

        Ok(())
    }

    #[test]
    fn fmt_test() {
        let config_str = include_str!("../../log_config.yml");
        let config = serde_yaml::from_str(config_str).unwrap();
        log4rs::init_raw_config(config).unwrap_or_else(|err| {
            error!("log4rs init error: {}", err);
        });
        let a = 10;
        let b = 20;
        let c = a + b;
        println!("{a},{b},{c}");
        error!("{a},{b},{c}");
    }

    #[test]
    fn test_plot_bar() -> Result<()> {
        let root = BitMapBackend::new("1.png", (640, 480)).into_drawing_area();
        root.fill(&WHITE)?;
        let mut chart = ChartBuilder::on(&root)
            .caption("y=x^2", ("sans-serif", 50).into_font())
            .margin(5)
            .x_label_area_size(30)
            .y_label_area_size(30)
            .build_cartesian_2d(-1f32..1f32, -0.1f32..1f32)?;

        chart.configure_mesh().draw()?;

        chart
            .draw_series(LineSeries::new(
                (-50..=50).map(|x| x as f32 / 50.0).map(|x| (x, x * x)),
                &RED,
            ))?
            .label("y = x^2")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

        chart
            .configure_series_labels()
            .background_style(&WHITE.mix(0.8))
            .border_style(&BLACK)
            .draw()?;

        root.present()?;
        Ok(())
    }

    #[test]
    fn plot_bin() {}
    #[test]
    fn plot_hist() {}
    #[test]
    fn plot_bar() -> Result<()> {
        let root = BitMapBackend::new("bar.png", (1024, 768)).into_drawing_area();

        root.fill(&WHITE)?;

        let sd = 0.13;

        let random_points: Vec<(f64, f64)> = {
            let norm_dist = Normal::new(0.5, sd).unwrap();
            let mut x_rand = XorShiftRng::from_seed(*b"MyFragileSeed123");
            let mut y_rand = XorShiftRng::from_seed(*b"MyFragileSeed321");
            let x_iter = norm_dist.sample_iter(&mut x_rand);
            let y_iter = norm_dist.sample_iter(&mut y_rand);
            x_iter.zip(y_iter).take(5000).collect()
        };

        let areas = root.split_by_breakpoints([944], [80]);

        let mut x_hist_ctx = ChartBuilder::on(&areas[0])
            .y_label_area_size(40)
            .build_cartesian_2d((0.0..1.0).step(0.01).use_round().into_segmented(), 0..250)?;
        let mut y_hist_ctx = ChartBuilder::on(&areas[3])
            .x_label_area_size(40)
            .build_cartesian_2d(0..250, (0.0..1.0).step(0.01).use_round())?;
        let mut scatter_ctx = ChartBuilder::on(&areas[2])
            .x_label_area_size(40)
            .y_label_area_size(40)
            .build_cartesian_2d(0f64..1f64, 0f64..1f64)?;
        scatter_ctx
            .configure_mesh()
            .disable_x_mesh()
            .disable_y_mesh()
            .draw()?;
        scatter_ctx.draw_series(
            random_points
                .iter()
                .map(|(x, y)| Circle::new((*x, *y), 2, GREEN.filled())),
        )?;
        let x_hist = Histogram::vertical(&x_hist_ctx)
            .style(GREEN.filled())
            .margin(0)
            .data(random_points.iter().map(|(x, _)| (*x, 1)));
        let y_hist = Histogram::horizontal(&y_hist_ctx)
            .style(GREEN.filled())
            .margin(0)
            .data(random_points.iter().map(|(_, y)| (*y, 1)));
        x_hist_ctx.draw_series(x_hist)?;
        y_hist_ctx.draw_series(y_hist)?;

        // To avoid the IO failure being ignored silently, we manually call the present function
        root.present().expect("Unable to write result to file, please make sure 'plotters-doc-data' dir exists under current dir");
        println!("Result has been saved to {}", "bar.png");

        Ok(())
    }
}
