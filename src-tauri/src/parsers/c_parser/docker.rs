use anyhow::{anyhow, Context, Result};
use log::{debug, error, info, warn};
use tokio::process::Command;
use tokio::fs;
use tempfile::tempdir;

/// Сервис для взаимодействия с Docker контейнером парсера C
pub struct DockerService {
    docker_available: bool,
}

impl DockerService {
    /// Создает новый экземпляр сервиса
    pub fn new() -> Self {
        Self {
            docker_available: Self::check_docker(),
        }
    }

    /// Проверяет наличие Docker (синхронная, вызывается при инициализации)
    fn check_docker() -> bool {
        let output = std::process::Command::new("docker")
            .args(&["info"])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                info!("Docker доступен");
                true
            }
            _ => {
                error!("Docker не найден. Убедитесь, что Docker установлен и запущен.");
                false
            }
        }
    }

    /// Проверяет, существует ли Docker образ
    pub async fn check_image_exists(image_name: &str) -> Result<bool> {
        let check_image = Command::new("docker")
            .args(&["image", "inspect", image_name])
            .output()
            .await?;

        Ok(check_image.status.success())
    }

    /// Запускает Docker контейнер и сразу удаляет после использования
    pub async fn run_container_clean(&self, code: &str, image_name: &str) -> Result<String> {
        // Создаем временный файл с кодом
        let temp_dir = tempdir()?;
        let input_file = temp_dir.path().join("input.c");
        fs::write(&input_file, code).await?;

        info!("Запуск Docker контейнера для парсинга C кода");
        debug!("Путь к временному файлу: {:?}", input_file);
        debug!("Содержимое файла:\n{}", code);

        // Проверяем, существует ли образ
        if !Self::check_image_exists(image_name).await? {
            error!("Образ {} не найден. Запустите ./build.sh в папке docker", image_name);
            return Err(anyhow!(
                "Образ Docker не найден. Соберите его: cd docker && ./build.sh"
            ));
        }

        // Формируем команду для запуска контейнера
        let container_command = format!(
            "docker run --rm -v {}:/input/input.c:ro {} python app.py /input/input.c",
            input_file.display(),
            image_name
        );
        info!("Выполнение команды: {}", container_command);

        let start = std::time::Instant::now();

        let output = Command::new("docker")
            .args(&[
                "run",
                "--rm",
                "-v",
                &format!("{}:/input/input.c:ro", input_file.display()),
                image_name,
                "python",
                "app.py",
                "/input/input.c",
            ])
            .output()
            .await
            .context("Ошибка запуска Docker контейнера")?;

        let elapsed = start.elapsed();
        info!("Docker контейнер выполнился за {:?}", elapsed);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);

            error!(
                "Docker контейнер завершился с ошибкой (код: {:?})",
                output.status.code()
            );
            error!("STDERR: {}", stderr);
            if !stdout.is_empty() {
                error!("STDOUT: {}", stdout);
            }

            return Err(anyhow!("Docker контейнер завершился с ошибкой: {}", stderr));
        }

        let stdout = String::from_utf8(output.stdout)?;
        debug!(
            "Ответ от парсера (первые 500 символов): {}",
            &stdout.chars().take(500).collect::<String>()
        );

        if stdout.is_empty() {
            warn!("Получен пустой ответ от парсера");
        }

        Ok(stdout)
    }

    /// Возвращает доступность Docker
    pub fn is_available(&self) -> bool {
        self.docker_available
    }
}

impl Default for DockerService {
    fn default() -> Self {
        Self::new()
    }
}
