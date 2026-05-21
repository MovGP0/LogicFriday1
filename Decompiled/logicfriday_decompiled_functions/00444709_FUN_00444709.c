/* 00444709 FUN_00444709 */

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_00444709(void)

{
  char cVar1;
  char cVar2;
  UINT CodePage;
  uint *_Str1;
  int iVar3;
  size_t sVar4;
  long lVar5;
  DWORD DVar6;
  uint *_Str;
  int local_8;
  
  __lock(7);
  CodePage = DAT_0046c980;
  DAT_00452254 = 0xffffffff;
  DAT_00452248 = 0xffffffff;
  DAT_0046c7d4 = 0;
  _Str1 = (uint *)FID_conflict___getenv_lk("TZ");
  if ((_Str1 == (uint *)0x0) || ((char)*_Str1 == '\0')) {
    if (DAT_0046c7d8 != (uint *)0x0) {
      _free(DAT_0046c7d8);
      DAT_0046c7d8 = (uint *)0x0;
    }
    FUN_00441cd6(7);
    DVar6 = GetTimeZoneInformation((LPTIME_ZONE_INFORMATION)&DAT_0046c728);
    if (DVar6 == 0xffffffff) {
      return;
    }
    _DAT_004521b0 = DAT_0046c728 * 0x3c;
    DAT_0046c7d4 = 1;
    if (DAT_0046c76e != 0) {
      _DAT_004521b0 = _DAT_004521b0 + DAT_0046c77c * 0x3c;
    }
    if ((DAT_0046c7c2 == 0) || (DAT_0046c7d0 == 0)) {
      DAT_004521b4 = 0;
      DAT_004521b8 = 0;
    }
    else {
      DAT_004521b8 = (DAT_0046c7d0 - DAT_0046c77c) * 0x3c;
      DAT_004521b4 = 1;
    }
    iVar3 = WideCharToMultiByte(CodePage,0,(LPCWSTR)&DAT_0046c72c,-1,PTR_DAT_00452240,0x3f,
                                (LPCSTR)0x0,&local_8);
    if ((iVar3 == 0) || (local_8 != 0)) {
      *PTR_DAT_00452240 = 0;
    }
    else {
      PTR_DAT_00452240[0x3f] = 0;
    }
    iVar3 = WideCharToMultiByte(CodePage,0,(LPCWSTR)&DAT_0046c780,-1,PTR_DAT_00452244,0x3f,
                                (LPCSTR)0x0,&local_8);
    if ((iVar3 != 0) && (local_8 == 0)) {
      PTR_DAT_00452244[0x3f] = 0;
      return;
    }
LAB_0044496e:
    *PTR_DAT_00452244 = 0;
  }
  else {
    if (DAT_0046c7d8 == (uint *)0x0) {
LAB_00444789:
      sVar4 = _strlen((char *)_Str1);
      DAT_0046c7d8 = _malloc(sVar4 + 1);
      if (DAT_0046c7d8 != (uint *)0x0) {
        FUN_0043ebd0(DAT_0046c7d8,_Str1);
        FUN_00441cd6(7);
        _strncpy(PTR_DAT_00452240,(char *)_Str1,3);
        _Str = (uint *)((int)_Str1 + 3);
        PTR_DAT_00452240[3] = 0;
        cVar1 = *(char *)_Str;
        if (cVar1 == '-') {
          _Str = _Str1 + 1;
        }
        lVar5 = _atol((char *)_Str);
        _DAT_004521b0 = lVar5 * 0xe10;
        for (; (cVar2 = (char)*_Str, cVar2 == '+' || (('/' < cVar2 && (cVar2 < ':'))));
            _Str = (uint *)((int)_Str + 1)) {
        }
        if ((char)*_Str == ':') {
          _Str = (uint *)((int)_Str + 1);
          lVar5 = _atol((char *)_Str);
          _DAT_004521b0 = _DAT_004521b0 + lVar5 * 0x3c;
          for (; ('/' < (char)*_Str && ((char)*_Str < ':')); _Str = (uint *)((int)_Str + 1)) {
          }
          if ((char)*_Str == ':') {
            _Str = (uint *)((int)_Str + 1);
            lVar5 = _atol((char *)_Str);
            _DAT_004521b0 = _DAT_004521b0 + lVar5;
            for (; ('/' < (char)*_Str && ((char)*_Str < ':')); _Str = (uint *)((int)_Str + 1)) {
            }
          }
        }
        if (cVar1 == '-') {
          _DAT_004521b0 = -_DAT_004521b0;
        }
        DAT_004521b4 = (int)(char)*_Str;
        if (DAT_004521b4 != 0) {
          _strncpy(PTR_DAT_00452244,(char *)_Str,3);
          PTR_DAT_00452244[3] = 0;
          return;
        }
        goto LAB_0044496e;
      }
    }
    else {
      iVar3 = _strcmp((char *)_Str1,(char *)DAT_0046c7d8);
      if (iVar3 != 0) {
        if (DAT_0046c7d8 != (uint *)0x0) {
          _free(DAT_0046c7d8);
        }
        goto LAB_00444789;
      }
    }
    FUN_00441cd6(7);
  }
  return;
}
