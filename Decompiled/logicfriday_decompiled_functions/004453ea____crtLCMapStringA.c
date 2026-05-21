/* 004453ea ___crtLCMapStringA */

/* WARNING: Function: __SEH_prolog replaced with injection: SEH_prolog */
/* WARNING: Function: __chkstk replaced with injection: alloca_probe */
/* WARNING: Function: __SEH_epilog replaced with injection: EH_epilog3 */
/* WARNING: Unable to track spacebase fully for stack */
/* Library Function - Single Match
    ___crtLCMapStringA
   
   Library: Visual Studio 2003 Release */

int __cdecl
___crtLCMapStringA(_locale_t _Plocinfo,LPCWSTR _LocaleName,DWORD _DwMapFlag,LPCSTR _LpSrcStr,
                  int _CchSrc,LPSTR _LpDestStr,int _CchDest,int _Code_page,BOOL _BError)

{
  int iVar1;
  DWORD DVar2;
  int iVar3;
  undefined1 *puVar4;
  int iVar5;
  LPSTR pCVar6;
  LPCSTR pCVar7;
  undefined1 *puVar8;
  undefined1 *puVar9;
  undefined1 *puVar10;
  undefined1 *puVar11;
  char *pcVar12;
  undefined4 uStackY_74;
  uint local_4c;
  size_t local_44;
  UINT local_40;
  int local_3c;
  LPSTR local_38;
  undefined1 *local_34;
  undefined1 *local_30;
  int local_2c;
  int local_28;
  int local_24;
  int local_20;
  undefined1 *local_1c;
  undefined4 uStack_c;
  undefined *local_8;
  
  local_8 = &DAT_0044e110;
  uStack_c = 0x4453f6;
  if (DAT_0046c804 == 0) {
    uStackY_74 = 0x445417;
    iVar1 = LCMapStringW(0,0x100,L"",1,(LPWSTR)0x0,0);
    if (iVar1 == 0) {
      DVar2 = GetLastError();
      if (DVar2 == 0x78) {
        DAT_0046c804 = 2;
      }
    }
    else {
      DAT_0046c804 = 1;
    }
  }
  pcVar12 = (char *)_DwMapFlag;
  pCVar7 = _LpSrcStr;
  if (0 < (int)_LpSrcStr) {
    do {
      pCVar7 = pCVar7 + -1;
      if (*pcVar12 == '\0') goto LAB_00445450;
      pcVar12 = pcVar12 + 1;
    } while (pCVar7 != (LPCSTR)0x0);
    pCVar7 = (LPCSTR)0xffffffff;
LAB_00445450:
    _LpSrcStr = _LpSrcStr + (-1 - (int)pCVar7);
  }
  if ((DAT_0046c804 != 2) && (DAT_0046c804 != 0)) {
    if (DAT_0046c804 == 1) {
      iVar1 = 0;
      local_20 = 0;
      local_24 = 0;
      local_28 = 0;
      if (_CchDest == 0) {
        _CchDest = DAT_0046c980;
      }
      uStackY_74 = 0x4454b0;
      iVar3 = MultiByteToWideChar(_CchDest,(uint)(_Code_page != 0) * 8 + 1,(LPCSTR)_DwMapFlag,
                                  (int)_LpSrcStr,(LPWSTR)0x0,0);
      local_2c = iVar3;
      if (iVar3 != 0) {
        puVar4 = (undefined1 *)(iVar3 * 2 + 3U & 0xfffffffc);
        iVar5 = -(int)puVar4;
        local_1c = &stack0xffffffa8 + iVar5;
        local_30 = &stack0xffffffa8 + iVar5;
        local_8 = (undefined *)0xffffffff;
        if (&stack0xffffffa8 == puVar4) {
          *(int *)(&stack0xffffffa4 + iVar5) = iVar3 * 2;
          *(undefined4 *)(&stack0xffffffa0 + iVar5) = 0x445509;
          local_30 = _malloc(*(size_t *)(&stack0xffffffa4 + iVar5));
          if (local_30 == (undefined1 *)0x0) {
            return 0;
          }
          local_24 = 1;
        }
        *(int *)(&stack0xffffffa4 + iVar5) = iVar3;
        *(undefined1 **)(&stack0xffffffa0 + iVar5) = local_30;
        *(LPCSTR *)(&stack0xffffff9c + iVar5) = _LpSrcStr;
        *(DWORD *)(&stack0xffffff98 + iVar5) = _DwMapFlag;
        *(undefined4 *)(&stack0xffffff94 + iVar5) = 1;
        *(int *)(&stack0xffffff90 + iVar5) = _CchDest;
        puVar8 = (undefined1 *)((int)&uStackY_74 + iVar5);
        *(undefined4 *)((int)&uStackY_74 + iVar5) = 0x445531;
        iVar5 = MultiByteToWideChar(*(UINT *)(&stack0xffffff90 + iVar5),
                                    *(DWORD *)(&stack0xffffff94 + iVar5),
                                    *(LPCSTR *)(&stack0xffffff98 + iVar5),
                                    *(int *)(&stack0xffffff9c + iVar5),
                                    *(LPWSTR *)(&stack0xffffffa0 + iVar5),
                                    *(int *)(&stack0xffffffa4 + iVar5));
        puVar4 = puVar8;
        if (iVar5 != 0) {
          *(undefined4 *)(puVar8 + -4) = 0;
          *(undefined4 *)(puVar8 + -8) = 0;
          *(int *)(puVar8 + -0xc) = iVar3;
          *(undefined1 **)(puVar8 + -0x10) = local_30;
          *(LPCWSTR *)(puVar8 + -0x14) = _LocaleName;
          *(_locale_t *)(puVar8 + -0x18) = _Plocinfo;
          puVar9 = puVar8 + -0x1c;
          *(undefined4 *)(puVar8 + -0x1c) = 0x44554b;
          iVar1 = LCMapStringW(*(LCID *)(puVar8 + -0x18),*(DWORD *)(puVar8 + -0x14),
                               *(LPCWSTR *)(puVar8 + -0x10),*(int *)(puVar8 + -0xc),
                               *(LPWSTR *)(puVar8 + -8),*(int *)(puVar8 + -4));
          local_20 = iVar1;
          puVar4 = puVar9;
          if (iVar1 != 0) {
            if (((uint)_LocaleName & 0x400) == 0) {
              puVar4 = (undefined1 *)(iVar1 * 2 + 3U & 0xfffffffc);
              *(undefined4 *)(puVar9 + -4) = 0x4455a0;
              iVar5 = -(int)puVar4;
              local_1c = puVar9 + iVar5;
              local_34 = puVar9 + iVar5;
              local_8 = (undefined *)0xffffffff;
              if (puVar9 == puVar4) {
                *(int *)(puVar9 + iVar5 + -4) = iVar1 * 2;
                *(undefined4 *)(puVar9 + iVar5 + -8) = 0x4455d7;
                local_34 = _malloc(*(size_t *)(puVar9 + iVar5 + -4));
                puVar4 = puVar9 + iVar5;
                if (local_34 == (undefined1 *)0x0) goto LAB_0044561f;
                local_28 = 1;
              }
              *(int *)(puVar9 + iVar5 + -4) = iVar1;
              *(undefined1 **)(puVar9 + iVar5 + -8) = local_34;
              *(int *)(puVar9 + iVar5 + -0xc) = iVar3;
              *(undefined1 **)(puVar9 + iVar5 + -0x10) = local_30;
              *(LPCWSTR *)(puVar9 + iVar5 + -0x14) = _LocaleName;
              *(_locale_t *)(puVar9 + iVar5 + -0x18) = _Plocinfo;
              puVar11 = puVar9 + iVar5 + -0x1c;
              *(undefined4 *)(puVar9 + iVar5 + -0x1c) = 0x4455fa;
              iVar3 = LCMapStringW(*(LCID *)(puVar9 + iVar5 + -0x18),
                                   *(DWORD *)(puVar9 + iVar5 + -0x14),
                                   *(LPCWSTR *)(puVar9 + iVar5 + -0x10),
                                   *(int *)(puVar9 + iVar5 + -0xc),*(LPWSTR *)(puVar9 + iVar5 + -8),
                                   *(int *)(puVar9 + iVar5 + -4));
              puVar4 = puVar11;
              if (iVar3 != 0) {
                *(undefined4 *)(puVar11 + -4) = 0;
                *(undefined4 *)(puVar11 + -8) = 0;
                if (_LpDestStr == (LPSTR)0x0) {
                  *(undefined4 *)(puVar11 + -0xc) = 0;
                  *(undefined4 *)(puVar11 + -0x10) = 0;
                }
                else {
                  *(LPSTR *)(puVar11 + -0xc) = _LpDestStr;
                  *(int *)(puVar11 + -0x10) = _CchSrc;
                }
                *(int *)(puVar11 + -0x14) = iVar1;
                *(undefined1 **)(puVar11 + -0x18) = local_34;
                *(undefined4 *)(puVar11 + -0x1c) = 0;
                *(int *)(puVar11 + -0x20) = _CchDest;
                puVar4 = puVar11 + -0x24;
                *(undefined4 *)(puVar11 + -0x24) = 0x44561d;
                iVar1 = WideCharToMultiByte(*(UINT *)(puVar11 + -0x20),*(DWORD *)(puVar11 + -0x1c),
                                            *(LPCWSTR *)(puVar11 + -0x18),*(int *)(puVar11 + -0x14),
                                            *(LPSTR *)(puVar11 + -0x10),*(int *)(puVar11 + -0xc),
                                            *(LPCSTR *)(puVar11 + -8),*(LPBOOL *)(puVar11 + -4));
              }
            }
            else if ((_LpDestStr != (LPSTR)0x0) && (iVar1 <= (int)_LpDestStr)) {
              *(LPSTR *)(puVar9 + -4) = _LpDestStr;
              *(int *)(puVar9 + -8) = _CchSrc;
              *(int *)(puVar9 + -0xc) = iVar3;
              *(undefined1 **)(puVar9 + -0x10) = local_30;
              *(LPCWSTR *)(puVar9 + -0x14) = _LocaleName;
              *(_locale_t *)(puVar9 + -0x18) = _Plocinfo;
              puVar10 = puVar9 + -0x1c;
              *(undefined4 *)(puVar9 + -0x1c) = 0x445586;
              LCMapStringW(*(LCID *)(puVar9 + -0x18),*(DWORD *)(puVar9 + -0x14),
                           *(LPCWSTR *)(puVar9 + -0x10),*(int *)(puVar9 + -0xc),
                           *(LPWSTR *)(puVar9 + -8),*(int *)(puVar9 + -4));
              puVar4 = puVar10;
            }
          }
        }
LAB_0044561f:
        if (local_28 != 0) {
          *(undefined1 **)(puVar4 + -4) = local_34;
          *(undefined4 *)(puVar4 + -8) = 0x44562c;
          _free(*(void **)(puVar4 + -4));
        }
        if (local_24 != 0) {
          *(undefined1 **)(puVar4 + -4) = local_30;
          *(undefined4 *)(puVar4 + -8) = 0x44563a;
          _free(*(void **)(puVar4 + -4));
          return iVar1;
        }
        return iVar1;
      }
    }
    return 0;
  }
  local_38 = (LPSTR)0x0;
  pcVar12 = (char *)0x0;
  local_3c = 0;
  if (_Plocinfo == (_locale_t)0x0) {
    _Plocinfo = DAT_0046c970;
  }
  if (_CchDest == 0) {
    _CchDest = DAT_0046c980;
  }
  local_40 = FUN_004481bf((LCID)_Plocinfo);
  if (local_40 == 0xffffffff) {
    return 0;
  }
  if (local_40 == _CchDest) {
    uStackY_74 = 0x44578b;
    local_4c = LCMapStringA((LCID)_Plocinfo,(DWORD)_LocaleName,(LPCSTR)_DwMapFlag,(int)_LpSrcStr,
                            (LPSTR)_CchSrc,(int)_LpDestStr);
    goto LAB_0044578d;
  }
  uStackY_74 = 0x445697;
  local_38 = FUN_00448208(_CchDest,local_40,(char *)_DwMapFlag,(size_t *)&_LpSrcStr,(LPSTR)0x0,0);
  if (local_38 == (LPSTR)0x0) {
    return 0;
  }
  uStackY_74 = 0x4456b3;
  local_44 = LCMapStringA((LCID)_Plocinfo,(DWORD)_LocaleName,local_38,(int)_LpSrcStr,(LPSTR)0x0,0);
  if (local_44 != 0) {
    local_8 = (undefined *)0x0;
    local_1c = &stack0xffffffa8;
    pcVar12 = &stack0xffffffa8;
    _memset(&stack0xffffffa8,0,local_44);
    local_8 = (undefined *)0xffffffff;
    if (&stack0x00000000 == (undefined1 *)0x58) {
      pcVar12 = _malloc(local_44);
      if (pcVar12 != (char *)0x0) {
        _memset(pcVar12,0,local_44);
        local_3c = 1;
        goto LAB_0044571e;
      }
    }
    else {
LAB_0044571e:
      uStackY_74 = 0x445734;
      local_44 = LCMapStringA((LCID)_Plocinfo,(DWORD)_LocaleName,local_38,(int)_LpSrcStr,pcVar12,
                              local_44);
      if (local_44 != 0) {
        uStackY_74 = 0x445755;
        pCVar6 = FUN_00448208(local_40,_CchDest,pcVar12,&local_44,(LPSTR)_CchSrc,(int)_LpDestStr);
        local_4c = (uint)(pCVar6 != (LPSTR)0x0);
        goto LAB_00445765;
      }
    }
    local_4c = 0;
  }
LAB_00445765:
  if (local_3c != 0) {
    _free(pcVar12);
  }
LAB_0044578d:
  if (local_38 != (LPSTR)0x0) {
    _free(local_38);
    return local_4c;
  }
  return local_4c;
}
