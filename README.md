# MintyBot

Discord 봇으로 ChatGPT API를 활용하여 멘션에 응답하는 봇입니다. 대화 기록을 유지하여 맥락에 맞는 응답을 제공합니다.

## 주요 기능

- Discord 멘션 감지 및 응답
- ChatGPT API를 통한 대화 처리
- 채널별 대화 기록 유지
- 긴 메시지 자동 분할 기능

## Docker Compose로 배포하기

### 준비 사항

1. Discord 봇 토큰
2. OpenAI API 키
3. Docker 및 Docker Compose 설치

### 배포 단계

1. 환경 변수 설정:
   ```bash
   cp .env.example .env
   # .env 파일을 편집하여 필요한 토큰 입력
   ```

2. Docker 이미지 빌드 및 실행:
   ```bash
   docker compose up -d
   ```

3. 로그 확인:
   ```bash
   docker compose logs -f
   ```

4. 봇 중지:
   ```bash
   docker compose down
   ```

### 환경 변수

- `MINTYBOT_DISCORD_TOKEN`: Discord 봇 토큰
- `MINTYBOT_OPENAI_TOKEN`: OpenAI API 키
- `MINTYBOT_DEV_USER_ID`: 개발자 Discord 사용자 ID (알림용)

## 개발 환경

- Rust (nightly-2025-04-24)
- Serenity Discord 라이브러리
- OpenAI API
