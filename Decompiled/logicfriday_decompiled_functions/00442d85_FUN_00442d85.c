/* 00442d85 FUN_00442d85 */

undefined4 FUN_00442d85(void)

{
  int iVar1;
  DWORD *lpTlsValue;
  BOOL BVar2;
  DWORD DVar3;
  
  iVar1 = __mtinitlocks();
  if (iVar1 != 0) {
    DAT_00452104 = TlsAlloc();
    if (DAT_00452104 != 0xffffffff) {
      lpTlsValue = _calloc(1,0x88);
      if (lpTlsValue != (DWORD *)0x0) {
        BVar2 = TlsSetValue(DAT_00452104,lpTlsValue);
        if (BVar2 != 0) {
          lpTlsValue[0x15] = (DWORD)&DAT_00452108;
          lpTlsValue[5] = 1;
          DVar3 = GetCurrentThreadId();
          lpTlsValue[1] = 0xffffffff;
          *lpTlsValue = DVar3;
          return 1;
        }
      }
      FUN_00442b78();
      return 0;
    }
  }
  FUN_00442b78();
  return 0;
}
