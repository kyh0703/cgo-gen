#include "is_aa_master_wrapper.h"
#include <cstdlib>
#include <cstring>
#include <new>
#include <string>

#include "IsAAMaster.h"

IsAAMasterHandle* sil_IsAAMaster_new(void) {
    return reinterpret_cast<IsAAMasterHandle*>(new IsAAMaster());
}

void sil_IsAAMaster_delete(IsAAMasterHandle* self) {
    delete reinterpret_cast<IsAAMaster*>(self);
}

uint32_t sil_IsAAMaster_GetAAMasterId(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetAAMasterId();
}

uint32_t sil_IsAAMaster_GetTenantId(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetTenantId();
}

uint32_t sil_IsAAMaster_GetNodeId(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetNodeId();
}

const char* sil_IsAAMaster_GetAADn(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetAADn();
}

const char* sil_IsAAMaster_GetAAName(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetAAName();
}

uint16_t sil_IsAAMaster_GetScenarioUseYn(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetScenarioUseYn();
}

uint32_t sil_IsAAMaster_GetScenarioId(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetScenarioId();
}

uint16_t sil_IsAAMaster_GetInitMentType(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetInitMentType();
}

uint32_t sil_IsAAMaster_GetInitMentId(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetInitMentId();
}

uint16_t sil_IsAAMaster_GetMenuMentType(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetMenuMentType();
}

uint32_t sil_IsAAMaster_GetMenuMentId(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetMenuMentId();
}

uint16_t sil_IsAAMaster_GetDigit1_Act(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetDigit1_Act();
}

const char* sil_IsAAMaster_GetDigit1_Num(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetDigit1_Num();
}

uint16_t sil_IsAAMaster_GetDigit2_Act(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetDigit2_Act();
}

const char* sil_IsAAMaster_GetDigit2_Num(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetDigit2_Num();
}

uint16_t sil_IsAAMaster_GetDigit3_Act(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetDigit3_Act();
}

const char* sil_IsAAMaster_GetDigit3_Num(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetDigit3_Num();
}

uint16_t sil_IsAAMaster_GetDigit4_Act(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetDigit4_Act();
}

const char* sil_IsAAMaster_GetDigit4_Num(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetDigit4_Num();
}

uint16_t sil_IsAAMaster_GetDigit5_Act(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetDigit5_Act();
}

const char* sil_IsAAMaster_GetDigit5_Num(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetDigit5_Num();
}

uint16_t sil_IsAAMaster_GetDigit6_Act(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetDigit6_Act();
}

const char* sil_IsAAMaster_GetDigit6_Num(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetDigit6_Num();
}

uint16_t sil_IsAAMaster_GetDigit7_Act(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetDigit7_Act();
}

const char* sil_IsAAMaster_GetDigit7_Num(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetDigit7_Num();
}

uint16_t sil_IsAAMaster_GetDigit8_Act(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetDigit8_Act();
}

const char* sil_IsAAMaster_GetDigit8_Num(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetDigit8_Num();
}

uint16_t sil_IsAAMaster_GetDigit9_Act(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetDigit9_Act();
}

const char* sil_IsAAMaster_GetDigit9_Num(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetDigit9_Num();
}

uint16_t sil_IsAAMaster_GetDigit0_Act(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetDigit0_Act();
}

const char* sil_IsAAMaster_GetDigit0_Num(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetDigit0_Num();
}

uint16_t sil_IsAAMaster_GetDigitA_Act(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetDigitA_Act();
}

const char* sil_IsAAMaster_GetDigitA_Num(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetDigitA_Num();
}

uint16_t sil_IsAAMaster_GetDigitS_Act(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetDigitS_Act();
}

const char* sil_IsAAMaster_GetDigitS_Num(IsAAMasterHandle* self) {
    return reinterpret_cast<IsAAMaster*>(self)->GetDigitS_Num();
}

void sil_IsAAMaster_SetAAMasterId(IsAAMasterHandle* self, uint32_t nAAMasterId) {
    reinterpret_cast<IsAAMaster*>(self)->SetAAMasterId(static_cast<uint32>(nAAMasterId));
}

void sil_IsAAMaster_SetTenantId(IsAAMasterHandle* self, uint32_t nTenantId) {
    reinterpret_cast<IsAAMaster*>(self)->SetTenantId(static_cast<uint32>(nTenantId));
}

void sil_IsAAMaster_SetNodeId(IsAAMasterHandle* self, uint32_t nNodeId) {
    reinterpret_cast<IsAAMaster*>(self)->SetNodeId(static_cast<uint32>(nNodeId));
}

void sil_IsAAMaster_SetAADn(IsAAMasterHandle* self, const char* sAADn) {
    reinterpret_cast<IsAAMaster*>(self)->SetAADn(sAADn);
}

void sil_IsAAMaster_SetAAName(IsAAMasterHandle* self, const char* sAAName) {
    reinterpret_cast<IsAAMaster*>(self)->SetAAName(sAAName);
}

void sil_IsAAMaster_SetScenarioUseYn(IsAAMasterHandle* self, uint16_t nScenarioUseYn) {
    reinterpret_cast<IsAAMaster*>(self)->SetScenarioUseYn(static_cast<uint16>(nScenarioUseYn));
}

void sil_IsAAMaster_SetScenarioId(IsAAMasterHandle* self, uint32_t nScenarioId) {
    reinterpret_cast<IsAAMaster*>(self)->SetScenarioId(static_cast<uint32>(nScenarioId));
}

void sil_IsAAMaster_SetInitMentType(IsAAMasterHandle* self, uint16_t nMentType) {
    reinterpret_cast<IsAAMaster*>(self)->SetInitMentType(static_cast<uint16>(nMentType));
}

void sil_IsAAMaster_SetInitMentId(IsAAMasterHandle* self, uint32_t nMentId) {
    reinterpret_cast<IsAAMaster*>(self)->SetInitMentId(static_cast<uint32>(nMentId));
}

void sil_IsAAMaster_SetMenuMentType(IsAAMasterHandle* self, uint16_t nMentType) {
    reinterpret_cast<IsAAMaster*>(self)->SetMenuMentType(static_cast<uint16>(nMentType));
}

void sil_IsAAMaster_SetMenuMentId(IsAAMasterHandle* self, uint32_t nMentId) {
    reinterpret_cast<IsAAMaster*>(self)->SetMenuMentId(static_cast<uint32>(nMentId));
}

void sil_IsAAMaster_SetDigit1_Act(IsAAMasterHandle* self, uint16_t nDigitAct) {
    reinterpret_cast<IsAAMaster*>(self)->SetDigit1_Act(static_cast<uint16>(nDigitAct));
}

void sil_IsAAMaster_SetDigit1_Num(IsAAMasterHandle* self, const char* sDigitNum) {
    reinterpret_cast<IsAAMaster*>(self)->SetDigit1_Num(sDigitNum);
}

void sil_IsAAMaster_SetDigit2_Act(IsAAMasterHandle* self, uint16_t nDigitAct) {
    reinterpret_cast<IsAAMaster*>(self)->SetDigit2_Act(static_cast<uint16>(nDigitAct));
}

void sil_IsAAMaster_SetDigit2_Num(IsAAMasterHandle* self, const char* sDigitNum) {
    reinterpret_cast<IsAAMaster*>(self)->SetDigit2_Num(sDigitNum);
}

void sil_IsAAMaster_SetDigit3_Act(IsAAMasterHandle* self, uint16_t nDigitAct) {
    reinterpret_cast<IsAAMaster*>(self)->SetDigit3_Act(static_cast<uint16>(nDigitAct));
}

void sil_IsAAMaster_SetDigit3_Num(IsAAMasterHandle* self, const char* sDigitNum) {
    reinterpret_cast<IsAAMaster*>(self)->SetDigit3_Num(sDigitNum);
}

void sil_IsAAMaster_SetDigit4_Act(IsAAMasterHandle* self, uint16_t nDigitAct) {
    reinterpret_cast<IsAAMaster*>(self)->SetDigit4_Act(static_cast<uint16>(nDigitAct));
}

void sil_IsAAMaster_SetDigit4_Num(IsAAMasterHandle* self, const char* sDigitNum) {
    reinterpret_cast<IsAAMaster*>(self)->SetDigit4_Num(sDigitNum);
}

void sil_IsAAMaster_SetDigit5_Act(IsAAMasterHandle* self, uint16_t nDigitAct) {
    reinterpret_cast<IsAAMaster*>(self)->SetDigit5_Act(static_cast<uint16>(nDigitAct));
}

void sil_IsAAMaster_SetDigit5_Num(IsAAMasterHandle* self, const char* sDigitNum) {
    reinterpret_cast<IsAAMaster*>(self)->SetDigit5_Num(sDigitNum);
}

void sil_IsAAMaster_SetDigit6_Act(IsAAMasterHandle* self, uint16_t nDigitAct) {
    reinterpret_cast<IsAAMaster*>(self)->SetDigit6_Act(static_cast<uint16>(nDigitAct));
}

void sil_IsAAMaster_SetDigit6_Num(IsAAMasterHandle* self, const char* sDigitNum) {
    reinterpret_cast<IsAAMaster*>(self)->SetDigit6_Num(sDigitNum);
}

void sil_IsAAMaster_SetDigit7_Act(IsAAMasterHandle* self, uint16_t nDigitAct) {
    reinterpret_cast<IsAAMaster*>(self)->SetDigit7_Act(static_cast<uint16>(nDigitAct));
}

void sil_IsAAMaster_SetDigit7_Num(IsAAMasterHandle* self, const char* sDigitNum) {
    reinterpret_cast<IsAAMaster*>(self)->SetDigit7_Num(sDigitNum);
}

void sil_IsAAMaster_SetDigit8_Act(IsAAMasterHandle* self, uint16_t nDigitAct) {
    reinterpret_cast<IsAAMaster*>(self)->SetDigit8_Act(static_cast<uint16>(nDigitAct));
}

void sil_IsAAMaster_SetDigit8_Num(IsAAMasterHandle* self, const char* sDigitNum) {
    reinterpret_cast<IsAAMaster*>(self)->SetDigit8_Num(sDigitNum);
}

void sil_IsAAMaster_SetDigit9_Act(IsAAMasterHandle* self, uint16_t nDigitAct) {
    reinterpret_cast<IsAAMaster*>(self)->SetDigit9_Act(static_cast<uint16>(nDigitAct));
}

void sil_IsAAMaster_SetDigit9_Num(IsAAMasterHandle* self, const char* sDigitNum) {
    reinterpret_cast<IsAAMaster*>(self)->SetDigit9_Num(sDigitNum);
}

void sil_IsAAMaster_SetDigit0_Act(IsAAMasterHandle* self, uint16_t nDigitAct) {
    reinterpret_cast<IsAAMaster*>(self)->SetDigit0_Act(static_cast<uint16>(nDigitAct));
}

void sil_IsAAMaster_SetDigit0_Num(IsAAMasterHandle* self, const char* sDigitNum) {
    reinterpret_cast<IsAAMaster*>(self)->SetDigit0_Num(sDigitNum);
}

void sil_IsAAMaster_SetDigitA_Act(IsAAMasterHandle* self, uint16_t nDigitAct) {
    reinterpret_cast<IsAAMaster*>(self)->SetDigitA_Act(static_cast<uint16>(nDigitAct));
}

void sil_IsAAMaster_SetDigitA_Num(IsAAMasterHandle* self, const char* sDigitNum) {
    reinterpret_cast<IsAAMaster*>(self)->SetDigitA_Num(sDigitNum);
}

void sil_IsAAMaster_SetDigitS_Act(IsAAMasterHandle* self, uint16_t nDigitAct) {
    reinterpret_cast<IsAAMaster*>(self)->SetDigitS_Act(static_cast<uint16>(nDigitAct));
}

void sil_IsAAMaster_SetDigitS_Num(IsAAMasterHandle* self, const char* sDigitNum) {
    reinterpret_cast<IsAAMaster*>(self)->SetDigitS_Num(sDigitNum);
}

