## 요약 (Summary)
- **관련 모듈**: `HWPX` / `DOCX` / `XLSX` / `PPTX` / `PDF Sign` / `Diagram DSL` / `MCP` / `VSIX·Native`
- **해결 이슈**: Closes #

## 변경 사항 상세 (What Changed)
- 

## 실측 검증 및 스크린샷 증명 (Empirical QA & Screenshots)
> UI, 도식화 레이아웃, 차트, PDF 서명 변경 시 **라이트 모드와 다크 모드 캡처**를 함께 첨부해 주세요.

| 라이트 모드 (Light Theme) | 다크 모드 (Dark Theme) |
| :---: | :---: |
| *(캡처 이미지 첨부)* | *(캡처 이미지 첨부)* |

## 필수 품질 체크리스트 (Quality & Clean-Room Gate)
- [ ] **Never Crash**: 프로덕션 Rust 코드에 `unsafe`, `unwrap()`, `expect()`, `panic!()`, `todo!()`가 없습니다.
- [ ] **무손실 라운드트립**: 문서 수정 → 저장 → 재열기 시 기존 텍스트/수식/링크/메타데이터 손실이 없습니다.
- [ ] **크로스플랫폼 & WASM**: OS 종속 API(Win32 COM 등)를 사용하지 않았으며 `wasm32-unknown-unknown` 빌드가 통과합니다.
- [ ] **비주얼 & 접근성**: 용지 순백색 유지, WCAG 명도 대비 `≥ 4.5:1`, 한글 조합형 IME 입력 정상 동작을 확인했습니다.
- [ ] **클린룸 & 라이선스**: 독점 에셋(MS/한컴/Adobe)이나 GPL/AGPL 코드를 포함하지 않았으며, 추가된 모든 이미지·샘플 파일을 `ATTRIBUTION.md`에 등재했습니다.
