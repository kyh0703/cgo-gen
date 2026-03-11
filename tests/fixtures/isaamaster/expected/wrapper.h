#ifndef SIL_WRAPPER_H
#define SIL_WRAPPER_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct IsAAMasterHandle IsAAMasterHandle;

IsAAMasterHandle* sil_IsAAMaster_new(void);
void sil_IsAAMaster_delete(IsAAMasterHandle* self);
uint32_t sil_IsAAMaster_GetAAMasterId(IsAAMasterHandle* self);
uint32_t sil_IsAAMaster_GetTenantId(IsAAMasterHandle* self);
uint32_t sil_IsAAMaster_GetNodeId(IsAAMasterHandle* self);
const char* sil_IsAAMaster_GetAADn(IsAAMasterHandle* self);
const char* sil_IsAAMaster_GetAAName(IsAAMasterHandle* self);
uint16_t sil_IsAAMaster_GetScenarioUseYn(IsAAMasterHandle* self);
uint32_t sil_IsAAMaster_GetScenarioId(IsAAMasterHandle* self);
uint16_t sil_IsAAMaster_GetInitMentType(IsAAMasterHandle* self);
uint32_t sil_IsAAMaster_GetInitMentId(IsAAMasterHandle* self);
uint16_t sil_IsAAMaster_GetMenuMentType(IsAAMasterHandle* self);
uint32_t sil_IsAAMaster_GetMenuMentId(IsAAMasterHandle* self);
uint16_t sil_IsAAMaster_GetDigit1_Act(IsAAMasterHandle* self);
const char* sil_IsAAMaster_GetDigit1_Num(IsAAMasterHandle* self);
uint16_t sil_IsAAMaster_GetDigit2_Act(IsAAMasterHandle* self);
const char* sil_IsAAMaster_GetDigit2_Num(IsAAMasterHandle* self);
uint16_t sil_IsAAMaster_GetDigit3_Act(IsAAMasterHandle* self);
const char* sil_IsAAMaster_GetDigit3_Num(IsAAMasterHandle* self);
uint16_t sil_IsAAMaster_GetDigit4_Act(IsAAMasterHandle* self);
const char* sil_IsAAMaster_GetDigit4_Num(IsAAMasterHandle* self);
uint16_t sil_IsAAMaster_GetDigit5_Act(IsAAMasterHandle* self);
const char* sil_IsAAMaster_GetDigit5_Num(IsAAMasterHandle* self);
uint16_t sil_IsAAMaster_GetDigit6_Act(IsAAMasterHandle* self);
const char* sil_IsAAMaster_GetDigit6_Num(IsAAMasterHandle* self);
uint16_t sil_IsAAMaster_GetDigit7_Act(IsAAMasterHandle* self);
const char* sil_IsAAMaster_GetDigit7_Num(IsAAMasterHandle* self);
uint16_t sil_IsAAMaster_GetDigit8_Act(IsAAMasterHandle* self);
const char* sil_IsAAMaster_GetDigit8_Num(IsAAMasterHandle* self);
uint16_t sil_IsAAMaster_GetDigit9_Act(IsAAMasterHandle* self);
const char* sil_IsAAMaster_GetDigit9_Num(IsAAMasterHandle* self);
uint16_t sil_IsAAMaster_GetDigit0_Act(IsAAMasterHandle* self);
const char* sil_IsAAMaster_GetDigit0_Num(IsAAMasterHandle* self);
uint16_t sil_IsAAMaster_GetDigitA_Act(IsAAMasterHandle* self);
const char* sil_IsAAMaster_GetDigitA_Num(IsAAMasterHandle* self);
uint16_t sil_IsAAMaster_GetDigitS_Act(IsAAMasterHandle* self);
const char* sil_IsAAMaster_GetDigitS_Num(IsAAMasterHandle* self);
void sil_IsAAMaster_SetAAMasterId(IsAAMasterHandle* self, uint32_t nAAMasterId);
void sil_IsAAMaster_SetTenantId(IsAAMasterHandle* self, uint32_t nTenantId);
void sil_IsAAMaster_SetNodeId(IsAAMasterHandle* self, uint32_t nNodeId);
void sil_IsAAMaster_SetAADn(IsAAMasterHandle* self, const char* sAADn);
void sil_IsAAMaster_SetAAName(IsAAMasterHandle* self, const char* sAAName);
void sil_IsAAMaster_SetScenarioUseYn(IsAAMasterHandle* self, uint16_t nScenarioUseYn);
void sil_IsAAMaster_SetScenarioId(IsAAMasterHandle* self, uint32_t nScenarioId);
void sil_IsAAMaster_SetInitMentType(IsAAMasterHandle* self, uint16_t nMentType);
void sil_IsAAMaster_SetInitMentId(IsAAMasterHandle* self, uint32_t nMentId);
void sil_IsAAMaster_SetMenuMentType(IsAAMasterHandle* self, uint16_t nMentType);
void sil_IsAAMaster_SetMenuMentId(IsAAMasterHandle* self, uint32_t nMentId);
void sil_IsAAMaster_SetDigit1_Act(IsAAMasterHandle* self, uint16_t nDigitAct);
void sil_IsAAMaster_SetDigit1_Num(IsAAMasterHandle* self, const char* sDigitNum);
void sil_IsAAMaster_SetDigit2_Act(IsAAMasterHandle* self, uint16_t nDigitAct);
void sil_IsAAMaster_SetDigit2_Num(IsAAMasterHandle* self, const char* sDigitNum);
void sil_IsAAMaster_SetDigit3_Act(IsAAMasterHandle* self, uint16_t nDigitAct);
void sil_IsAAMaster_SetDigit3_Num(IsAAMasterHandle* self, const char* sDigitNum);
void sil_IsAAMaster_SetDigit4_Act(IsAAMasterHandle* self, uint16_t nDigitAct);
void sil_IsAAMaster_SetDigit4_Num(IsAAMasterHandle* self, const char* sDigitNum);
void sil_IsAAMaster_SetDigit5_Act(IsAAMasterHandle* self, uint16_t nDigitAct);
void sil_IsAAMaster_SetDigit5_Num(IsAAMasterHandle* self, const char* sDigitNum);
void sil_IsAAMaster_SetDigit6_Act(IsAAMasterHandle* self, uint16_t nDigitAct);
void sil_IsAAMaster_SetDigit6_Num(IsAAMasterHandle* self, const char* sDigitNum);
void sil_IsAAMaster_SetDigit7_Act(IsAAMasterHandle* self, uint16_t nDigitAct);
void sil_IsAAMaster_SetDigit7_Num(IsAAMasterHandle* self, const char* sDigitNum);
void sil_IsAAMaster_SetDigit8_Act(IsAAMasterHandle* self, uint16_t nDigitAct);
void sil_IsAAMaster_SetDigit8_Num(IsAAMasterHandle* self, const char* sDigitNum);
void sil_IsAAMaster_SetDigit9_Act(IsAAMasterHandle* self, uint16_t nDigitAct);
void sil_IsAAMaster_SetDigit9_Num(IsAAMasterHandle* self, const char* sDigitNum);
void sil_IsAAMaster_SetDigit0_Act(IsAAMasterHandle* self, uint16_t nDigitAct);
void sil_IsAAMaster_SetDigit0_Num(IsAAMasterHandle* self, const char* sDigitNum);
void sil_IsAAMaster_SetDigitA_Act(IsAAMasterHandle* self, uint16_t nDigitAct);
void sil_IsAAMaster_SetDigitA_Num(IsAAMasterHandle* self, const char* sDigitNum);
void sil_IsAAMaster_SetDigitS_Act(IsAAMasterHandle* self, uint16_t nDigitAct);
void sil_IsAAMaster_SetDigitS_Num(IsAAMasterHandle* self, const char* sDigitNum);
#ifdef __cplusplus
}
#endif

#endif /* SIL_WRAPPER_H */
