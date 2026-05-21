/* 004483d7 FUN_004483d7 */

/* WARNING: Function: __chkstk replaced with injection: alloca_probe */

BOOL FUN_004483d7(void)

{
  SIZE_T SVar1;
  BOOL BVar2;
  LPCVOID lpAddress;
  PVOID lpAddress_00;
  undefined4 uStack_5c;
  _SYSTEM_INFO local_4c;
  _MEMORY_BASIC_INFORMATION local_28;
  DWORD local_c;
  SIZE_T local_8;
  
  uStack_5c = 0x4483e8;
  SVar1 = VirtualQuery(&uStack_5c,&local_28,0x1c);
  if (SVar1 != 0) {
    GetSystemInfo(&local_4c);
    lpAddress_00 = (PVOID)((~(local_4c.dwPageSize - 1) & (uint)&uStack_5c) - local_4c.dwPageSize);
    local_8 = local_4c.dwPageSize;
    if ((LPVOID)((-(uint)(DAT_0046c6e0 != 1) & 0xffff1000) + 0x11000 + (int)local_28.AllocationBase)
        <= lpAddress_00) {
      lpAddress = local_28.AllocationBase;
      if (DAT_0046c6e0 != 1) {
        do {
          SVar1 = VirtualQuery(lpAddress,&local_28,0x1c);
          if (SVar1 == 0) {
            return 0;
          }
          lpAddress = (LPCVOID)((int)lpAddress + local_28.RegionSize);
        } while ((local_28.State & 0x1000) == 0);
        if ((local_28.Protect._1_1_ & 1) != 0) {
          return 1;
        }
        if (lpAddress_00 < local_28.BaseAddress) {
          return 0;
        }
        VirtualAlloc(local_28.BaseAddress,local_8,0x1000,4);
        lpAddress_00 = local_28.BaseAddress;
      }
      BVar2 = VirtualProtect(lpAddress_00,local_8,(-(uint)(DAT_0046c6e0 != 1) & 0x103) + 1,&local_c)
      ;
      return BVar2;
    }
  }
  return 0;
}
