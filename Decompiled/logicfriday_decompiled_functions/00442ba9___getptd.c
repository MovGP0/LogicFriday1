/* 00442ba9 __getptd */

/* Library Function - Single Match
    __getptd
   
   Library: Visual Studio 2003 Release */

_ptiddata __cdecl __getptd(void)

{
  DWORD dwErrCode;
  _ptiddata lpTlsValue;
  BOOL BVar1;
  DWORD DVar2;
  
  dwErrCode = GetLastError();
  lpTlsValue = TlsGetValue(DAT_00452104);
  if (lpTlsValue == (_ptiddata)0x0) {
    lpTlsValue = _calloc(1,0x88);
    if (lpTlsValue != (_ptiddata)0x0) {
      BVar1 = TlsSetValue(DAT_00452104,lpTlsValue);
      if (BVar1 != 0) {
        lpTlsValue->_initaddr = &DAT_00452108;
        lpTlsValue->_holdrand = 1;
        DVar2 = GetCurrentThreadId();
        lpTlsValue->_thandle = 0xffffffff;
        lpTlsValue->_tid = DVar2;
        goto LAB_00442c0e;
      }
    }
    __amsg_exit(0x10);
  }
LAB_00442c0e:
  SetLastError(dwErrCode);
  return lpTlsValue;
}
