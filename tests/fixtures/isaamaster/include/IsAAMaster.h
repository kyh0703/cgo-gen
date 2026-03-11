#pragma once
#include <string.h>
#include <iRecSet.h>
#include <iLib.h>
#include "iSilDbData.h"

class IsAAMaster
{
public:
    IsAAMaster(void);
    ~IsAAMaster(void);

    inline uint32 GetAAMasterId(void) { return mDBData.nAAMasterId; }
    inline uint32 GetTenantId(void) { return mDBData.nTenantId; }
    inline uint32 GetNodeId(void) { return mDBData.nNodeId; }
    inline NPCSTR GetAADn(void) { return mDBData.sAADn; }
    inline NPCSTR GetAAName(void) { return mDBData.sAAName; }
    inline uint16 GetScenarioUseYn(void) { return mDBData.nScenarioUseYn; }
    inline uint32 GetScenarioId(void) { return mDBData.nScenarioId; }
    inline uint16 GetInitMentType(void) { return mDBData.nInitMentType; }
    inline uint32 GetInitMentId(void) { return mDBData.nInitMentId; }
    inline uint16 GetMenuMentType(void) { return mDBData.nMenuMentType; }
    inline uint32 GetMenuMentId(void) { return mDBData.nMenuMentId; }
    inline uint16 GetDigit1_Act(void) { return mDBData.nDigit1_Act; }
    inline NPCSTR GetDigit1_Num(void) { return mDBData.sDigit1_Num; }
    inline uint16 GetDigit2_Act(void) { return mDBData.nDigit2_Act; }
    inline NPCSTR GetDigit2_Num(void) { return mDBData.sDigit2_Num; }
    inline uint16 GetDigit3_Act(void) { return mDBData.nDigit3_Act; }
    inline NPCSTR GetDigit3_Num(void) { return mDBData.sDigit3_Num; }
    inline uint16 GetDigit4_Act(void) { return mDBData.nDigit4_Act; }
    inline NPCSTR GetDigit4_Num(void) { return mDBData.sDigit4_Num; }
    inline uint16 GetDigit5_Act(void) { return mDBData.nDigit5_Act; }
    inline NPCSTR GetDigit5_Num(void) { return mDBData.sDigit5_Num; }
    inline uint16 GetDigit6_Act(void) { return mDBData.nDigit6_Act; }
    inline NPCSTR GetDigit6_Num(void) { return mDBData.sDigit6_Num; }
    inline uint16 GetDigit7_Act(void) { return mDBData.nDigit7_Act; }
    inline NPCSTR GetDigit7_Num(void) { return mDBData.sDigit7_Num; }
    inline uint16 GetDigit8_Act(void) { return mDBData.nDigit8_Act; }
    inline NPCSTR GetDigit8_Num(void) { return mDBData.sDigit8_Num; }
    inline uint16 GetDigit9_Act(void) { return mDBData.nDigit9_Act; }
    inline NPCSTR GetDigit9_Num(void) { return mDBData.sDigit9_Num; }
    inline uint16 GetDigit0_Act(void) { return mDBData.nDigit0_Act; }
    inline NPCSTR GetDigit0_Num(void) { return mDBData.sDigit0_Num; }
    inline uint16 GetDigitA_Act(void) { return mDBData.nDigitA_Act; }
    inline NPCSTR GetDigitA_Num(void) { return mDBData.sDigitA_Num; }
    inline uint16 GetDigitS_Act(void) { return mDBData.nDigitS_Act; }
    inline NPCSTR GetDigitS_Num(void) { return mDBData.sDigitS_Num; }

    inline void SetAAMasterId(uint32 nAAMasterId) { mDBData.nAAMasterId = nAAMasterId; }
    inline void SetTenantId(uint32 nTenantId) { mDBData.nTenantId = nTenantId; }
    inline void SetNodeId(uint32 nNodeId) { mDBData.nNodeId = nNodeId; }
    inline void SetAADn(NPCSTR sAADn) { iStrCpy(mDBData.sAADn, sAADn); }
    inline void SetAAName(NPCSTR sAAName) { iStrCpy(mDBData.sAAName, sAAName); }
    inline void SetScenarioUseYn(uint16 nScenarioUseYn) { mDBData.nScenarioUseYn = nScenarioUseYn; }
    inline void SetScenarioId(uint32 nScenarioId) { mDBData.nScenarioId = nScenarioId; }
    inline void SetInitMentType(uint16 nMentType) { mDBData.nInitMentType = nMentType; }
    inline void SetInitMentId(uint32 nMentId) { mDBData.nInitMentId = nMentId; }
    inline void SetMenuMentType(uint16 nMentType) { mDBData.nMenuMentType = nMentType; }
    inline void SetMenuMentId(uint32 nMentId) { mDBData.nMenuMentId = nMentId; }
    inline void SetDigit1_Act(uint16 nDigitAct) { mDBData.nDigit1_Act = nDigitAct; }
    inline void SetDigit1_Num(NPCSTR sDigitNum) { iStrCpy(mDBData.sDigit1_Num, sDigitNum); }
    inline void SetDigit2_Act(uint16 nDigitAct) { mDBData.nDigit2_Act = nDigitAct; }
    inline void SetDigit2_Num(NPCSTR sDigitNum) { iStrCpy(mDBData.sDigit2_Num, sDigitNum); }
    inline void SetDigit3_Act(uint16 nDigitAct) { mDBData.nDigit3_Act = nDigitAct; }
    inline void SetDigit3_Num(NPCSTR sDigitNum) { iStrCpy(mDBData.sDigit3_Num, sDigitNum); }
    inline void SetDigit4_Act(uint16 nDigitAct) { mDBData.nDigit4_Act = nDigitAct; }
    inline void SetDigit4_Num(NPCSTR sDigitNum) { iStrCpy(mDBData.sDigit4_Num, sDigitNum); }
    inline void SetDigit5_Act(uint16 nDigitAct) { mDBData.nDigit5_Act = nDigitAct; }
    inline void SetDigit5_Num(NPCSTR sDigitNum) { iStrCpy(mDBData.sDigit5_Num, sDigitNum); }
    inline void SetDigit6_Act(uint16 nDigitAct) { mDBData.nDigit6_Act = nDigitAct; }
    inline void SetDigit6_Num(NPCSTR sDigitNum) { iStrCpy(mDBData.sDigit6_Num, sDigitNum); }
    inline void SetDigit7_Act(uint16 nDigitAct) { mDBData.nDigit7_Act = nDigitAct; }
    inline void SetDigit7_Num(NPCSTR sDigitNum) { iStrCpy(mDBData.sDigit7_Num, sDigitNum); }
    inline void SetDigit8_Act(uint16 nDigitAct) { mDBData.nDigit8_Act = nDigitAct; }
    inline void SetDigit8_Num(NPCSTR sDigitNum) { iStrCpy(mDBData.sDigit8_Num, sDigitNum); }
    inline void SetDigit9_Act(uint16 nDigitAct) { mDBData.nDigit9_Act = nDigitAct; }
    inline void SetDigit9_Num(NPCSTR sDigitNum) { iStrCpy(mDBData.sDigit9_Num, sDigitNum); }
    inline void SetDigit0_Act(uint16 nDigitAct) { mDBData.nDigit0_Act = nDigitAct; }
    inline void SetDigit0_Num(NPCSTR sDigitNum) { iStrCpy(mDBData.sDigit0_Num, sDigitNum); }
    inline void SetDigitA_Act(uint16 nDigitAct) { mDBData.nDigitA_Act = nDigitAct; }
    inline void SetDigitA_Num(NPCSTR sDigitNum) { iStrCpy(mDBData.sDigitA_Num, sDigitNum); }
    inline void SetDigitS_Act(uint16 nDigitAct) { mDBData.nDigitS_Act = nDigitAct; }
    inline void SetDigitS_Num(NPCSTR sDigitNum) { iStrCpy(mDBData.sDigitS_Num, sDigitNum); }

private:
    TB_IE_AA_MASTER mDBData;
};
