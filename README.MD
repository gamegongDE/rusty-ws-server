# RUSTY WS SERVER

## RELEASE NOTES

### 0.1.0
- 멀티스레드 기반 서버 코어
  - 단일 update 로직 스레드
  - 다중 tokio (warp) 웹소켓 스레드
- 세션 + 플레이어 구조
- 게임오브젝트 추상 구조

## WORK IN PROGRESS
- update 함수 고도화
  - update 내에서도 gameobject 생성 가능하도록 개선
- session 및 player 연결 단 구축
- 재접속 처리
- redis 연동