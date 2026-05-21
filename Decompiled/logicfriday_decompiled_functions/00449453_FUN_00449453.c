/* 00449453 FUN_00449453 */

/* WARNING: Function: __SEH_prolog replaced with injection: SEH_prolog */
/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */
/* WARNING: Function: __SEH_epilog replaced with injection: EH_epilog3 */
/* WARNING: Function: __chkstk replaced with injection: alloca_probe */
/* WARNING: Unable to track spacebase fully for stack */
/* WARNING: This function may have set the stack pointer */

int __cdecl
FUN_00449453(LCID param_1,DWORD param_2,byte *param_3,char *param_4,byte *param_5,char *param_6,
            UINT param_7)

{
  byte *pbVar1;
  int iVar2;
  DWORD DVar3;
  BOOL BVar4;
  BYTE *pBVar5;
  undefined1 *puVar6;
  undefined1 *puVar7;
  int iVar8;
  undefined1 *puVar9;
  undefined1 *puVar10;
  int iVar11;
  UINT UVar12;
  UINT UVar13;
  undefined1 *puVar14;
  undefined1 *puVar15;
  undefined1 *puVar16;
  size_t unaff_EDI;
  byte *_Memory;
  uint unaff_retaddr;
  undefined4 uStackY_7c;
  byte *local_54;
  _cpinfo local_40;
  int local_2c;
  int local_28;
  int local_24;
  uint local_20;
  undefined1 *local_1c;
  undefined4 uStack_c;
  undefined *local_8;
  
  local_8 = &DAT_0044eb08;
  uStack_c = 0x44945f;
  local_20 = DAT_00451a00 ^ unaff_retaddr;
  _Memory = (byte *)0x0;
  if (DAT_0046c9e0 == 0) {
    uStackY_7c = 0x449488;
    iVar2 = CompareStringW(0,0,L"",1,L"",1);
    if (iVar2 == 0) {
      DVar3 = GetLastError();
      if (DVar3 == 0x78) {
        DAT_0046c9e0 = 2;
      }
    }
    else {
      DAT_0046c9e0 = 1;
    }
  }
  if (0 < (int)param_4) {
    param_4 = (char *)_strncnt(param_4,unaff_EDI);
  }
  if (0 < (int)param_6) {
    param_6 = (char *)_strncnt(param_6,unaff_EDI);
  }
  if ((DAT_0046c9e0 == 2) || (DAT_0046c9e0 == 0)) {
    local_54 = (byte *)0x0;
    if (param_1 == 0) {
      param_1 = DAT_0046c970;
    }
    UVar13 = param_7;
    if (param_7 == 0) {
      UVar13 = DAT_0046c980;
    }
    UVar12 = FUN_004481bf(param_1);
    if (UVar12 != 0xffffffff) {
      pbVar1 = param_5;
      if (UVar12 == UVar13) {
LAB_00449799:
        param_5 = pbVar1;
        uStackY_7c = 0x4497b1;
        iVar2 = CompareStringA(param_1,param_2,(PCNZCH)param_3,(int)param_4,(PCNZCH)param_5,
                               (int)param_6);
        if (_Memory != (byte *)0x0) {
          _free(_Memory);
          _free(local_54);
          return iVar2;
        }
        return iVar2;
      }
      uStackY_7c = 0x449762;
      _Memory = (byte *)FUN_00448208(UVar13,UVar12,(char *)param_3,(size_t *)&param_4,(LPSTR)0x0,0);
      if (_Memory != (byte *)0x0) {
        uStackY_7c = 0x44977d;
        local_54 = (byte *)FUN_00448208(UVar13,UVar12,(char *)param_5,(size_t *)&param_6,(LPSTR)0x0,
                                        0);
        param_3 = _Memory;
        pbVar1 = local_54;
        if (local_54 != (byte *)0x0) goto LAB_00449799;
        _free(_Memory);
      }
    }
  }
  else if (DAT_0046c9e0 == 1) {
    local_24 = 0;
    local_28 = 0;
    local_2c = 0;
    if (param_7 == 0) {
      param_7 = DAT_0046c980;
    }
    if ((param_4 == (char *)0x0) || (param_6 == (char *)0x0)) {
      if (param_4 == param_6) {
        return 2;
      }
      if (1 < (int)param_6) {
        return 1;
      }
      if (1 < (int)param_4) {
        return 3;
      }
      BVar4 = GetCPInfo(param_7,&local_40);
      if (BVar4 == 0) {
        return 0;
      }
      if (0 < (int)param_4) {
        if (local_40.MaxCharSize < 2) {
          return 3;
        }
        pBVar5 = local_40.LeadByte;
        while( true ) {
          if (local_40.LeadByte[0] == 0) {
            return 3;
          }
          if (pBVar5[1] == 0) break;
          if ((*pBVar5 <= *param_3) && (*param_3 <= pBVar5[1])) {
            return 2;
          }
          pBVar5 = pBVar5 + 2;
          local_40.LeadByte[0] = *pBVar5;
        }
        return 3;
      }
      if (0 < (int)param_6) {
        if (local_40.MaxCharSize < 2) {
          return 1;
        }
        pBVar5 = local_40.LeadByte;
        if (local_40.LeadByte[0] != 0) {
          while( true ) {
            if (pBVar5[1] == 0) {
              return 1;
            }
            if ((*pBVar5 <= *param_5) && (*param_5 <= pBVar5[1])) break;
            pBVar5 = pBVar5 + 2;
            if (*pBVar5 == 0) {
              return 1;
            }
          }
          return 2;
        }
        return 1;
      }
    }
    uStackY_7c = 0x4495d2;
    iVar2 = MultiByteToWideChar(param_7,9,(LPCSTR)param_3,(int)param_4,(LPWSTR)0x0,0);
    if (iVar2 != 0) {
      puVar6 = (undefined1 *)(iVar2 * 2 + 3U & 0xfffffffc);
      iVar8 = -(int)puVar6;
      puVar7 = &stack0xffffffa0 + iVar8;
      local_1c = &stack0xffffffa0 + iVar8;
      local_8 = (undefined *)0xffffffff;
      if (&stack0xffffffa0 == puVar6) {
        *(int *)(&stack0xffffff9c + iVar8) = iVar2 * 2;
        *(undefined4 *)(&stack0xffffff98 + iVar8) = 0x44962a;
        puVar7 = _malloc(*(size_t *)(&stack0xffffff9c + iVar8));
        if (puVar7 == (undefined1 *)0x0) {
          return 0;
        }
        local_24 = 1;
      }
      *(int *)(&stack0xffffff9c + iVar8) = iVar2;
      *(undefined1 **)(&stack0xffffff98 + iVar8) = puVar7;
      *(char **)(&stack0xffffff94 + iVar8) = param_4;
      *(byte **)(&stack0xffffff90 + iVar8) = param_3;
      *(undefined4 *)(&stack0xffffff8c + iVar8) = 1;
      *(UINT *)(&stack0xffffff88 + iVar8) = param_7;
      puVar14 = (undefined1 *)((int)&uStackY_7c + iVar8);
      *(undefined4 *)((int)&uStackY_7c + iVar8) = 0x44964b;
      iVar8 = MultiByteToWideChar(*(UINT *)(&stack0xffffff88 + iVar8),
                                  *(DWORD *)(&stack0xffffff8c + iVar8),
                                  *(LPCSTR *)(&stack0xffffff90 + iVar8),
                                  *(int *)(&stack0xffffff94 + iVar8),
                                  *(LPWSTR *)(&stack0xffffff98 + iVar8),
                                  *(int *)(&stack0xffffff9c + iVar8));
      puVar6 = puVar14;
      if (iVar8 != 0) {
        *(undefined4 *)(puVar14 + -4) = 0;
        *(undefined4 *)(puVar14 + -8) = 0;
        *(char **)(puVar14 + -0xc) = param_6;
        *(byte **)(puVar14 + -0x10) = param_5;
        *(undefined4 *)(puVar14 + -0x14) = 9;
        *(UINT *)(puVar14 + -0x18) = param_7;
        puVar15 = puVar14 + -0x1c;
        *(undefined4 *)(puVar14 + -0x1c) = 0x449668;
        iVar8 = MultiByteToWideChar(*(UINT *)(puVar14 + -0x18),*(DWORD *)(puVar14 + -0x14),
                                    *(LPCSTR *)(puVar14 + -0x10),*(int *)(puVar14 + -0xc),
                                    *(LPWSTR *)(puVar14 + -8),*(int *)(puVar14 + -4));
        puVar6 = puVar15;
        if (iVar8 != 0) {
          puVar9 = (undefined1 *)(iVar8 * 2 + 3U & 0xfffffffc);
          *(undefined4 *)(puVar15 + -4) = 0x44968a;
          iVar11 = -(int)puVar9;
          puVar10 = puVar15 + iVar11;
          puVar6 = puVar15 + iVar11;
          local_1c = puVar15 + iVar11;
          local_8 = (undefined *)0xffffffff;
          if (puVar15 == puVar9) {
            sRamfffffffc = iVar8 * 2;
            uRamfffffff8 = 0x4496bd;
            puVar10 = _malloc(sRamfffffffc);
            if (puVar10 == (undefined1 *)0x0) goto LAB_00449704;
            local_28 = 1;
          }
          *(int *)(puVar15 + iVar11 + -4) = iVar8;
          *(undefined1 **)(puVar15 + iVar11 + -8) = puVar10;
          *(char **)(puVar15 + iVar11 + -0xc) = param_6;
          *(byte **)(puVar15 + iVar11 + -0x10) = param_5;
          *(undefined4 *)(puVar15 + iVar11 + -0x14) = 1;
          *(UINT *)(puVar15 + iVar11 + -0x18) = param_7;
          puVar16 = puVar15 + iVar11 + -0x1c;
          *(undefined4 *)(puVar15 + iVar11 + -0x1c) = 0x4496de;
          iVar11 = MultiByteToWideChar(*(UINT *)(puVar15 + iVar11 + -0x18),
                                       *(DWORD *)(puVar15 + iVar11 + -0x14),
                                       *(LPCSTR *)(puVar15 + iVar11 + -0x10),
                                       *(int *)(puVar15 + iVar11 + -0xc),
                                       *(LPWSTR *)(puVar15 + iVar11 + -8),
                                       *(int *)(puVar15 + iVar11 + -4));
          puVar6 = puVar16;
          if (iVar11 != 0) {
            *(int *)(puVar16 + -4) = iVar8;
            *(undefined1 **)(puVar16 + -8) = puVar10;
            *(int *)(puVar16 + -0xc) = iVar2;
            *(undefined1 **)(puVar16 + -0x10) = puVar7;
            *(DWORD *)(puVar16 + -0x14) = param_2;
            *(LCID *)(puVar16 + -0x18) = param_1;
            puVar6 = puVar16 + -0x1c;
            *(undefined4 *)(puVar16 + -0x1c) = 0x4496f4;
            local_2c = CompareStringW(*(LCID *)(puVar16 + -0x18),*(DWORD *)(puVar16 + -0x14),
                                      *(PCNZWCH *)(puVar16 + -0x10),*(int *)(puVar16 + -0xc),
                                      *(PCNZWCH *)(puVar16 + -8),*(int *)(puVar16 + -4));
          }
          if (local_28 != 0) {
            *(undefined1 **)(puVar6 + -4) = puVar10;
            *(undefined4 *)(puVar6 + -8) = 0x449703;
            _free(*(void **)(puVar6 + -4));
          }
        }
      }
LAB_00449704:
      if (local_24 != 0) {
        *(undefined1 **)(puVar6 + -4) = puVar7;
        *(undefined4 *)(puVar6 + -8) = 0x449712;
        _free(*(void **)(puVar6 + -4));
        return local_2c;
      }
      return local_2c;
    }
  }
  return 0;
}
