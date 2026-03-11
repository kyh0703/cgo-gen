#pragma once

#include "iSiDef.h"
#include <iType.h>
#include <string>
#include <valueDef.h>

typedef void (*SqlAct)(NPCSTR, uint32, uint16);

typedef struct
{
    uint32 nAAMasterId;
    uint32 nTenantId;
    uint32 nNodeId;
    char sAADn[SIL_DN_SIZE];
    char sAAName[SIL_NAME128];
    uint16 nScenarioUseYn;
    uint32 nScenarioId;
    uint16 nInitMentType;
    uint32 nInitMentId;
    uint16 nMenuMentType;
    uint32 nMenuMentId;
    uint16 nDigit1_Act;
    char sDigit1_Num[SIL_DNIS_SIZE];
    uint16 nDigit2_Act;
    char sDigit2_Num[SIL_DNIS_SIZE];
    uint16 nDigit3_Act;
    char sDigit3_Num[SIL_DNIS_SIZE];
    uint16 nDigit4_Act;
    char sDigit4_Num[SIL_DNIS_SIZE];
    uint16 nDigit5_Act;
    char sDigit5_Num[SIL_DNIS_SIZE];
    uint16 nDigit6_Act;
    char sDigit6_Num[SIL_DNIS_SIZE];
    uint16 nDigit7_Act;
    char sDigit7_Num[SIL_DNIS_SIZE];
    uint16 nDigit8_Act;
    char sDigit8_Num[SIL_DNIS_SIZE];
    uint16 nDigit9_Act;
    char sDigit9_Num[SIL_DNIS_SIZE];
    uint16 nDigit0_Act;
    char sDigit0_Num[SIL_DNIS_SIZE];
    uint16 nDigitA_Act;
    char sDigitA_Num[SIL_DNIS_SIZE];
    uint16 nDigitS_Act;
    char sDigitS_Num[SIL_DNIS_SIZE];
} TB_IE_AA_MASTER;
