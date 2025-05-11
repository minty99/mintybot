# MintyBot

Discord 봇으로 OpenAI API를 활용하여 멘션에 응답하는 봇입니다. 대화 기록을 유지하여 맥락에 맞는 응답을 제공합니다.

## 주요 기능

- Discord 멘션 감지 및 응답 (사용자 멘션 및 역할 멘션 지원)
- OpenAI API를 통한 대화 처리 (Responses API 활용)
- 채널별 대화 기록 유지 및 컨텍스트 관리
- 긴 메시지 자동 분할 기능
- 상세한 로깅 시스템 (대화 내용, 토큰 사용량 등)
- 관리자 명령어 지원

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
   docker compose up -d --build
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
- `MINTYBOT_DEV_USER_ID`: 개발자 Discord 사용자 ID (알림 및 `<dev>` 명령어 사용)

## 로깅 시스템

- 모든 대화는 `data/logs/conversations.log`에 기록됩니다
- 로그에는 다음 정보가 포함됩니다:
  - 채널 ID, 길드 ID, 길드 이름, 채널 이름
  - KST 타임스탬프
  - API 호출 소요 시간
  - 요청 대화 내용
  - OpenAI 응답
  - 토큰 사용량
