/* 00449a6b __mbschr */

/* Library Function - Single Match
    __mbschr
   
   Library: Visual Studio 2003 Release */

uchar * __cdecl __mbschr(uchar *_Str,uint _Ch)

{
  byte bVar1;
  byte bVar2;
  _ptiddata p_Var3;
  pthreadmbcinfo ptVar4;
  uchar *puVar5;
  byte *pbVar6;
  uint uVar7;
  
  p_Var3 = __getptd();
  ptVar4 = p_Var3->_tpxcptinfoptrs;
  if (ptVar4 != DAT_0046c9f8) {
    ptVar4 = ___updatetmbcinfo();
  }
  if (ptVar4->ismbcodepage == 0) {
    puVar5 = (uchar *)_strchr((char *)_Str,_Ch);
    return puVar5;
  }
  while( true ) {
    bVar2 = *_Str;
    uVar7 = (uint)bVar2;
    if (bVar2 == 0) break;
    if ((ptVar4->mbctype[uVar7 + 5] & 4) == 0) {
      pbVar6 = _Str;
      if (_Ch == uVar7) break;
    }
    else {
      bVar1 = _Str[1];
      if (bVar1 == 0) {
        return (uchar *)0x0;
      }
      pbVar6 = _Str + 1;
      if (_Ch == CONCAT11(bVar2,bVar1)) {
        return _Str;
      }
    }
    _Str = pbVar6 + 1;
  }
  return (uchar *)(~-(uint)(_Ch != uVar7) & (uint)_Str);
}



/* 00449af0 KERNEL32.DLL::RtlUnwind */

void RtlUnwind(PVOID TargetFrame,PVOID TargetIp,PEXCEPTION_RECORD ExceptionRecord,PVOID ReturnValue)

{
                    /* WARNING: Could not recover jumptable at 0x00449af0. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  RtlUnwind(TargetFrame,TargetIp,ExceptionRecord,ReturnValue);
  return;
}
