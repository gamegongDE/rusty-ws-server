# RUSTY WS SERVER

## RELEASE NOTES

### 0.2.0

- 멀티스레드 구조 변경
  - 단일 update 로직 태스크
  - 최대 16개의 네트워크 처리 태스크
- msgpack 기능 추가
- 미사용 코드 및 종속성 제거
- update 함수 고도화
  - update 내에서도 gameobject 생성 가능

### 0.1.0

- 멀티스레드 기반 서버 코어
  - 단일 update 로직 스레드
  - 다중 tokio (warp) 웹소켓 스레드
- 세션 + 플레이어 구조
- 게임오브젝트 추상 구조
