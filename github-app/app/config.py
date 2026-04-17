import os


class Settings:
    GITHUB_APP_ID: str = os.environ["GITHUB_APP_ID"]
    GITHUB_PRIVATE_KEY: str = os.environ["GITHUB_PRIVATE_KEY"]
    GITHUB_WEBHOOK_SECRET: str = os.environ["GITHUB_WEBHOOK_SECRET"]
    HF_TOKEN: str = os.environ["HF_TOKEN"]
    LOOPSLEUTH_API_URL: str = os.environ["LOOPSLEUTH_API_URL"]
    # Max files to analyze per PR (to avoid very long runs)
    MAX_FILES: int = int(os.environ.get("LOOPSLEUTH_MAX_FILES", "15"))


settings = Settings()
